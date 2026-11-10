"""Inspect one explicitly selected local document without changing it"""
import argparse
import codecs
import json
import os
from pathlib import Path
import stat
import sys
import subprocess
from .scanner import EXCLUDED, MARKERS

TEXT_EXTENSIONS = {".txt", ".md", ".csv", ".log"}
MAX_BYTES = 2 * 1024 * 1024
MAX_CHARS = 16000
DIRECTORY_FLAGS = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW


class InspectionBlocked(Exception):
    pass


def _is_dataless(info):
    # SF_DATALESS is also checked on Python versions before the named constant
    return bool(getattr(info, "st_flags", 0) & getattr(stat, "SF_DATALESS", 0x40000000))


def _excluded(name):
    return name in EXCLUDED or name.startswith((".env", ".venv")) or name.lower().endswith(
        (".key", ".pem", ".p12", ".keychain-db", ".app", ".bundle", ".photoslibrary"))


def _identity(info):
    return info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns


def _check_directory(fd):
    if _is_dataless(os.fstat(fd)):
        raise InspectionBlocked("dataless_directory")
    for marker in MARKERS:
        try:
            info = os.stat(marker, dir_fd=fd, follow_symlinks=False)
        except FileNotFoundError:
            continue
        if stat.S_ISREG(info.st_mode) or (marker == ".git" and stat.S_ISDIR(info.st_mode)):
            raise InspectionBlocked("project_boundary")


def _open_root(root):
    fd = os.open(root.anchor, DIRECTORY_FLAGS)
    try:
        for part in root.parts[1:]:
            if _excluded(part):
                raise InspectionBlocked("excluded_path")
            before = os.stat(part, dir_fd=fd, follow_symlinks=False)
            if not stat.S_ISDIR(before.st_mode):
                raise InspectionBlocked("unsafe_directory")
            if _is_dataless(before):
                raise InspectionBlocked("dataless_directory")
            child = os.open(part, DIRECTORY_FLAGS, dir_fd=fd)
            if (os.fstat(child).st_dev, os.fstat(child).st_ino) != (before.st_dev, before.st_ino):
                os.close(child)
                raise InspectionBlocked("directory_changed")
            os.close(fd)
            fd = child
        return fd
    except BaseException:
        os.close(fd)
        raise


def _read_snapshot(root, parts, max_bytes):
    fd = _open_root(root)
    try:
        _check_directory(fd)
        for part in parts[:-1]:
            before = os.stat(part, dir_fd=fd, follow_symlinks=False)
            if not stat.S_ISDIR(before.st_mode):
                raise InspectionBlocked("unsafe_directory")
            if _is_dataless(before):
                raise InspectionBlocked("dataless_directory")
            child = os.open(part, DIRECTORY_FLAGS, dir_fd=fd)
            opened = os.fstat(child)
            if (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino):
                os.close(child)
                raise InspectionBlocked("directory_changed")
            os.close(fd)
            fd = child
            _check_directory(fd)
        before = os.stat(parts[-1], dir_fd=fd, follow_symlinks=False)
        if not stat.S_ISREG(before.st_mode):
            raise InspectionBlocked("not_regular_file")
        if before.st_nlink != 1:
            raise InspectionBlocked("hardlinked_file")
        if _is_dataless(before):
            raise InspectionBlocked("dataless_file")
        if before.st_size > max_bytes:
            raise InspectionBlocked("byte_limit")
        document = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=fd)
        try:
            if _identity(os.fstat(document)) != _identity(before):
                raise InspectionBlocked("file_changed")
            chunks, total = [], 0
            while True:
                chunk = os.read(document, min(65536, max_bytes + 1 - total))
                if not chunk:
                    break
                chunks.append(chunk)
                total += len(chunk)
                if total > max_bytes:
                    raise InspectionBlocked("byte_limit")
            after = os.fstat(document)
            if _identity(before) != _identity(after) or total != before.st_size:
                raise InspectionBlocked("file_changed")
            return b"".join(chunks)
        finally:
            os.close(document)
    finally:
        os.close(fd)


def _inspect_pdf(payload, max_pages, max_chars):
    if not payload.startswith(b"%PDF-"):
        return dict(status="skipped", reason="invalid_pdf_signature")
    try:
        process = subprocess.run(
            [sys.executable, "-I", "-B", str(Path(__file__).with_name("_pdf_worker.py")), str(max_pages), str(max_chars)],
            input=payload, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            env={"PATH": os.defpath}, close_fds=True, timeout=5)
        if process.returncode or len(process.stdout) > 200000:
            return dict(status="skipped", reason="pdf_worker_failed")
        result = json.loads(process.stdout)
        if not isinstance(result, dict) or result.get("status") not in {"ok", "no_text", "skipped"}:
            raise ValueError("Invalid worker response")
        if result["status"] in {"ok", "no_text"}:
            if not isinstance(result.get("text"), str) or len(result["text"]) > max_chars:
                raise ValueError("Invalid worker text")
        return result
    except subprocess.TimeoutExpired:
        return dict(status="skipped", reason="pdf_timeout")
    except (OSError, ValueError):
        return dict(status="skipped", reason="pdf_worker_failed")


def inspect_document(root, relative_path, *, include_text=False, max_bytes=262144, max_chars=MAX_CHARS,
                     enable_pdf=False, max_pages=5):
    if type(max_bytes) is not int or not 1 <= max_bytes <= MAX_BYTES:
        raise ValueError("max_bytes must be between 1 and 2097152")
    if type(max_chars) is not int or not 1 <= max_chars <= MAX_CHARS:
        raise ValueError("max_chars must be between 1 and 16000")
    if type(include_text) is not bool:
        raise ValueError("include_text must be boolean")
    if type(enable_pdf) is not bool or type(max_pages) is not int or not 1 <= max_pages <= 20:
        raise ValueError("PDF options must include a boolean and between 1 and 20 pages")
    relative_path = os.fspath(relative_path)
    if not relative_path or relative_path.startswith("/") or "\\" in relative_path or "\x00" in relative_path:
        raise ValueError("Provide a relative document path")
    parts = relative_path.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise ValueError("Path traversal is not allowed")
    root = Path(os.path.abspath(os.fspath(root)))
    if root == Path(root.anchor):
        raise ValueError("Choose a specific test folder rather than the filesystem root")
    result = dict(schema_version="0.1.0", report_type="document_inspection", path=relative_path,
                  read_only=True, content_is_untrusted=True, review_required=True,
                  status="skipped", reason=None, characters=0, text_included=False,
                  truncated=False, encoding=None, limits=dict(max_bytes=max_bytes, max_chars=max_chars))
    if any(_excluded(part) for part in parts):
        result["reason"] = "excluded_path"
        return result
    is_pdf = Path(parts[-1]).suffix.lower() == ".pdf"
    if is_pdf and not enable_pdf:
        result["reason"] = "pdf_not_enabled"
        return result
    if not is_pdf and Path(parts[-1]).suffix.lower() not in TEXT_EXTENSIONS:
        result["reason"] = "unsupported_type"
        return result
    try:
        payload = _read_snapshot(root, parts, max_bytes)
    except InspectionBlocked as exc:
        result["reason"] = str(exc)
        return result
    except OSError as exc:
        result.update(reason="unavailable_or_unsafe_path", error_code=type(exc).__name__)
        return result
    if is_pdf:
        parsed = _inspect_pdf(payload, max_pages, max_chars)
        text = parsed.pop("text", "")
        result.update(parsed, parser_process_used=True, characters=len(text))
        result["limits"]["max_pages"] = max_pages
        if include_text and result["status"] in {"ok", "no_text"}:
            result.update(text=text, text_included=True)
        return result
    encoding = "utf-16" if payload.startswith((codecs.BOM_UTF16_LE, codecs.BOM_UTF16_BE)) else "utf-8-sig"
    try:
        text = payload.decode(encoding, errors="strict")
    except UnicodeError:
        result["reason"] = "unsupported_or_invalid_encoding"
        return result
    if any(ord(char) < 32 and char not in "\t\r\n" for char in text):
        result["reason"] = "binary_or_control_data"
        return result
    result.update(status="ok" if text.strip() else "no_text", reason=None if text.strip() else "empty_document",
                  characters=min(len(text), max_chars), truncated=len(text) > max_chars, encoding=encoding)
    if include_text:
        result.update(text=text[:max_chars], text_included=True)
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", help="Explicit local test directory")
    parser.add_argument("path", help="Document path relative to the chosen directory")
    parser.add_argument("--include-text", action="store_true", help="Include extracted text in the local JSON report")
    parser.add_argument("--max-bytes", type=int, default=262144)
    parser.add_argument("--max-chars", type=int, default=MAX_CHARS)
    parser.add_argument("--pdf", action="store_true", help="Enable optional PDF inspection")
    parser.add_argument("--max-pages", type=int, default=5)
    args = parser.parse_args(argv)
    try:
        report = inspect_document(args.root, args.path, include_text=args.include_text,
                                  max_bytes=args.max_bytes, max_chars=args.max_chars,
                                  enable_pdf=args.pdf, max_pages=args.max_pages)
    except (OSError, ValueError) as exc:
        print(json.dumps(dict(error=type(exc).__name__, reason=str(exc))), file=sys.stderr)
        return 2
    print(json.dumps(report, indent=2))
    return 0 if report["status"] in {"ok", "no_text"} else 1


if __name__ == "__main__":
    sys.exit(main())
