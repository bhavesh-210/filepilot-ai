"""Dependency-free checks for report structure and safety invariants.

Validation never reads the filesystem. Version 0.1.0 remains the report contract;
0.1.2 tightens validation and corrects project-root labeling without adding fields.
"""
from collections import Counter
from datetime import datetime

CATEGORIES = {"Education", "Development", "Professional", "Finance", "Media",
              "Software", "Personal", "Unknown"}


def validate_report(report):
    """Raise ValueError for malformed reports, otherwise return None."""
    def require(condition, message):
        if not condition:
            raise ValueError("Invalid report: " + message)

    def fields(value, names):
        require(isinstance(value, dict), "expected object")
        require(set(value) == set(names.split()), "unexpected or missing fields")

    def relative(value, allow_root=False):
        require(isinstance(value, str) and bool(value), "empty path")
        if allow_root and value == ".":
            return
        require(not value.startswith("/") and "\x00" not in value,
                "invalid relative path")
        require(all(p not in {"", ".", ".."} for p in value.split("/")),
                "noncanonical relative path")

    def integer(value):
        return type(value) is int

    fields(report, "schema_version scanner_version root generated_at read_only analysis limits entries projects errors truncated summary")
    require(report["schema_version"] == "0.1.0", "unsupported schema")
    require(isinstance(report["scanner_version"], str) and bool(report["scanner_version"]), "scanner version")
    require(isinstance(report["root"], str) and report["root"].startswith("/"), "absolute root required")
    require(isinstance(report["generated_at"], str), "timestamp type")
    try:
        timestamp = datetime.fromisoformat(report["generated_at"])
        require(timestamp.utcoffset() is not None and timestamp.utcoffset().total_seconds() == 0, "UTC timestamp required")
    except (TypeError, ValueError) as exc:
        raise ValueError("Invalid report: timestamp") from exc
    require(report["read_only"] is True, "read_only must be true")
    require(report["analysis"] == "filename_and_metadata_only", "analysis mode")
    require(type(report["truncated"]) is bool, "truncated type")
    fields(report["limits"], "max_entries max_depth")
    limits = report["limits"]
    require(integer(limits["max_entries"]) and limits["max_entries"] > 0, "entry limit")
    require(integer(limits["max_depth"]) and 0 <= limits["max_depth"] <= 128, "depth limit")
    for name in ("entries", "projects", "errors"):
        require(isinstance(report[name], list), name + " must be a list")
    roots = set()
    for project in report["projects"]:
        fields(project, "path markers policy")
        relative(project["path"], True)
        require(project["path"] not in roots, "duplicate project")
        roots.add(project["path"])
        require(project["policy"] == "preserve_structure", "project policy")
        require(isinstance(project["markers"], list) and bool(project["markers"]), "project markers")
        for marker in project["markers"]:
            relative(marker)
            require("/" not in marker, "marker must be a basename")
    paths = set()
    for entry in report["entries"]:
        fields(entry, "path kind extension file_type size_bytes mtime_ns project_root status skip_reason classification")
        relative(entry["path"])
        require(entry["path"] not in paths, "duplicate entry")
        paths.add(entry["path"])
        require(entry["kind"] in {"file", "directory", "symlink", "special"}, "entry kind")
        require(isinstance(entry["extension"], str) and isinstance(entry["file_type"], str), "file types")
        require(integer(entry["size_bytes"]) and entry["size_bytes"] >= 0, "size")
        require(integer(entry["mtime_ns"]), "mtime")
        require(entry["status"] in {"inventoried", "skipped", "error"}, "entry status")
        require((entry["skip_reason"] is None) == (entry["status"] == "inventoried"), "status/reason mismatch")
        require(entry["skip_reason"] is None or isinstance(entry["skip_reason"], str), "skip reason")
        project = entry["project_root"]
        require(project is None or (isinstance(project, str) and project in roots), "unregistered project")
        if project is not None and project != ".":
            require(entry["path"] == project or entry["path"].startswith(project + "/"), "outside project boundary")
        for root in roots:
            if root == "." or entry["path"] == root or entry["path"].startswith(root + "/"):
                require(project == root, "missing project protection")
        c = entry["classification"]
        fields(c, "category reason recommendation suggested_category review_required")
        require(c["category"] in CATEGORIES, "category")
        require(isinstance(c["reason"], str) and bool(c["reason"]), "classification reason")
        require(c["recommendation"] in {"leave_untouched", "review", "suggest_category"}, "recommendation")
        require(type(c["review_required"]) is bool, "review flag")
        if c["recommendation"] == "suggest_category":
            require(c["suggested_category"] == c["category"] != "Unknown" and c["review_required"], "suggestion must be reviewed")
        else:
            require(c["suggested_category"] is None, "unexpected destination")
            require(c["review_required"] == (c["recommendation"] == "review"), "review invariant")
        if project is not None:
            require(c["category"] == "Development" and c["recommendation"] == "leave_untouched", "project policy violation")
        if entry["kind"] != "file" or entry["status"] != "inventoried":
            require(c["recommendation"] == "leave_untouched", "unsafe recommendation")
    require(len(paths) <= limits["max_entries"], "entry limit exceeded")
    for error in report["errors"]:
        fields(error, "path code errno")
        relative(error["path"], True)
        require(isinstance(error["code"], str), "error code")
        require(error["errno"] is None or integer(error["errno"]), "errno")
    summary = report["summary"]
    fields(summary, "entry_count project_count error_count categories")
    for name, values in (("entry_count", paths), ("project_count", roots), ("error_count", report["errors"])):
        require(integer(summary[name]) and summary[name] == len(values), "summary count mismatch")
    require(summary["categories"] == dict(Counter(e["classification"]["category"] for e in report["entries"])), "category counts mismatch")
