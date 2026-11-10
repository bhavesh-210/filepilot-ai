"""Emit a report to stdout only; redirect outside the scanned root if desired."""
import argparse
import json
import sys
from .scanner import scan


def main():
    parser = argparse.ArgumentParser(description="Read-only, offline FilePilot inventory")
    parser.add_argument("root", help="Explicit test directory; no default")
    parser.add_argument("--max-entries", type=int, default=100_000)
    parser.add_argument("--max-depth", type=int, default=64)
    args = parser.parse_args()
    try:
        report = scan(args.root, max_entries=args.max_entries, max_depth=args.max_depth)
    except (OSError, ValueError) as exc:
        print(json.dumps({"error": type(exc).__name__, "errno": getattr(exc, "errno", None)}), file=sys.stderr)
        return 2
    json.dump(report, sys.stdout, indent=2, ensure_ascii=True)
    print()
    return 1 if report["errors"] or report["truncated"] else 0


if __name__ == "__main__":
    sys.exit(main())
