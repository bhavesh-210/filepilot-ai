"""Metadata-only scanner. No file content reads, execution, or mutations.

Directory descriptors and O_NOFOLLOW prevent symlink substitution during descent.
This is a best-effort inventory, not an atomic filesystem snapshot.
"""
from __future__ import annotations

import os
import re
import stat
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from . import __version__
from .report import validate_report

CATEGORIES = ("Education", "Development", "Professional", "Finance", "Media",
              "Software", "Personal", "Unknown")
MARKERS = {".git", "package.json", "pyproject.toml", "Cargo.toml", "go.mod",
           "pom.xml", "build.gradle", "Package.swift", "CMakeLists.txt"}
EXCLUDED = {".ssh", ".aws", ".gnupg", ".git", ".Trash", "node_modules",
            ".venv", "venv", "__pycache__", "Library", "System"}
TYPES = {
    "source": {".py", ".rs", ".js", ".ts", ".tsx", ".jsx", ".java", ".c", ".cpp", ".swift", ".go"},
    "document": {".pdf", ".doc", ".docx", ".txt", ".md", ".ppt", ".pptx", ".odt"},
    "spreadsheet": {".csv", ".xlsx", ".xls", ".ods"},
    "image": {".png", ".jpg", ".jpeg", ".gif", ".heic", ".svg", ".webp"},
    "audio": {".mp3", ".wav", ".m4a", ".flac"},
    "video": {".mp4", ".mov", ".mkv"},
    "software": {".dmg", ".pkg", ".app", ".exe"},
    "archive": {".zip", ".tar", ".gz", ".7z"},
}


def classify(name: str, kind: str, project: str | None = None) -> dict:
    """Rule evidence is heuristic, never a calibrated model probability."""
    if project is not None:
        return {"category": "Development", "reason": "Inside detected project; preserve structure",
                "recommendation": "leave_untouched", "suggested_category": None,
                "review_required": False}
    # Source artifacts take precedence over their business-domain vocabulary.
    if kind == "source":
        return {"category": "Development", "reason": "Source file type rule",
                "recommendation": "suggest_category", "suggested_category": "Development",
                "review_required": True}
    tokens = set(re.findall(r"[a-z]+", Path(name).stem.lower()))
    matches = []
    for category, keywords in {
        "Education": {"lecture", "assignment", "exam", "college", "syllabus", "homework"},
        "Finance": {"invoice", "receipt", "tax", "bank", "statement", "salary"},
        "Professional": {"resume", "cv", "contract", "meeting", "proposal"},
        "Personal": {"diary", "personal", "journal"},
    }.items():
        if tokens & keywords:
            matches.append(category)
    if len(matches) > 1:
        category, reason = "Unknown", "Conflicting filename rules"
    elif matches:
        category, reason = matches[0], "Filename keyword rule"
    else:
        category = {"source": "Development", "image": "Media", "audio": "Media",
                    "video": "Media", "software": "Software"}.get(kind, "Unknown")
        reason = "Extension rule" if category != "Unknown" else "Insufficient metadata evidence"
    return {"category": category, "reason": reason,
            "recommendation": "review" if category == "Unknown" else "suggest_category",
            "suggested_category": None if category == "Unknown" else category,
            "review_required": True}


def scan(root: str | Path, *, max_entries: int = 100_000, max_depth: int = 64) -> dict:
    """Scan only an explicit root, never follow links, cap work, return JSON data.

    Projects are inventoried but all descendants inherit leave-untouched policy.
    Absolute roots containing symlink components are rejected as well.
    """
    if max_entries < 1 or not 0 <= max_depth <= 128:
        raise ValueError("max_entries must be positive; max_depth must be 0..128")
    path = Path(os.path.abspath(os.fspath(root)))
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    fd = os.open(path.anchor, flags)
    try:
        for part in path.parts[1:]:
            child = os.open(part, flags, dir_fd=fd)
            os.close(fd)
            fd = child
    except BaseException:
        os.close(fd)
        raise
    report = {"schema_version": "0.1.0", "scanner_version": __version__,
              "root": str(path), "generated_at": datetime.now(timezone.utc).isoformat(),
              "read_only": True, "analysis": "filename_and_metadata_only",
              "limits": {"max_entries": max_entries, "max_depth": max_depth},
              "entries": [], "projects": [], "errors": [], "truncated": False}
    entries = report["entries"]

    def error(rel, exc):
        report["errors"].append({"path": rel, "code": type(exc).__name__,
                                 "errno": getattr(exc, "errno", None)})

    def walk(directory_fd, relative, depth, inherited):
        try:
            names = sorted(os.listdir(directory_fd))
        except OSError as exc:
            error(relative or ".", exc)
            return
        markers = []
        for name in names:
            if name in MARKERS:
                try:
                    mode = os.stat(name, dir_fd=directory_fd, follow_symlinks=False).st_mode
                    if stat.S_ISREG(mode) or (name == ".git" and stat.S_ISDIR(mode)):
                        markers.append(name)
                except OSError as exc:
                    error(f"{relative}/{name}".lstrip("/"), exc)
        project = inherited
        if markers and inherited is None:
            project = relative or "."
            report["projects"].append({"path": project, "markers": markers,
                                       "policy": "preserve_structure"})
        for name in names:
            if len(entries) >= max_entries:
                report["truncated"] = True
                return
            rel = f"{relative}/{name}".lstrip("/")
            try:
                info = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            except OSError as exc:
                error(rel, exc)
                continue
            mode = info.st_mode
            kind = "symlink" if stat.S_ISLNK(mode) else "directory" if stat.S_ISDIR(mode) else "file" if stat.S_ISREG(mode) else "special"
            extension = Path(name).suffix.lower()
            file_type = next((key for key, extensions in TYPES.items() if extension in extensions), "unknown") if kind == "file" else kind
            excluded = name in EXCLUDED or name.startswith(".env") or extension in {".key", ".pem", ".p12", ".keychain-db"}
            skip = "sensitive_or_generated" if excluded else "symlink_not_followed" if kind == "symlink" else "special_file" if kind == "special" else "package_bundle" if kind == "directory" and extension in {".app", ".photoslibrary", ".bundle"} else None
            entry = {"path": rel, "kind": kind, "extension": extension,
                     "file_type": file_type, "size_bytes": info.st_size,
                     "mtime_ns": info.st_mtime_ns, "project_root": project,
                     "status": "skipped" if skip else "inventoried", "skip_reason": skip,
                     "classification": classify(name, file_type, project)}
            if skip or kind != "file":
                entry["classification"].update(recommendation="leave_untouched", suggested_category=None, review_required=False)
            entries.append(entry)
            if kind == "directory" and not skip:
                if depth >= max_depth:
                    entry.update(status="skipped", skip_reason="depth_limit")
                    report["truncated"] = True
                    continue
                try:
                    child = os.open(name, flags, dir_fd=directory_fd)
                    try:
                        opened = os.fstat(child)
                        if (opened.st_dev, opened.st_ino) != (info.st_dev, info.st_ino):
                            raise OSError("Directory changed during scan")
                        walk(child, rel, depth + 1, project)
                    finally:
                        os.close(child)
                except OSError as exc:
                    entry.update(status="error", skip_reason="directory_unavailable")
                    error(rel, exc)
    try:
        walk(fd, "", 0, None)
    finally:
        os.close(fd)
    # A project's own entry is emitted before its markers are inspected.
    # Reconcile it after traversal, including partial inventories.
    project_roots = {p["path"] for p in report["projects"]}
    for entry in entries:
        if entry["kind"] == "directory" and entry["path"] in project_roots:
            entry["project_root"] = entry["path"]
            entry["classification"] = classify(entry["path"], "directory", entry["path"])
    report["summary"] = {"entry_count": len(entries), "project_count": len(report["projects"]),
                         "error_count": len(report["errors"]),
                         "categories": dict(Counter(e["classification"]["category"] for e in entries))}
    validate_report(report)
    return report
