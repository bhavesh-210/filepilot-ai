"""Validate synthetic dataset records without accessing named files or networking."""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import sys
from .report import CATEGORIES

FIELDS = {"schema_version", "id", "family_id", "split", "filename", "file_type",
          "category", "expected_policy", "label_reason", "provenance", "reviewed"}
TYPES = {"document", "spreadsheet", "source", "image", "audio", "video",
         "software", "archive", "unknown"}
POLICIES = {"review", "suggest_category", "leave_untouched"}


def validate_records(records):
    """Require unique IDs/names, family isolation, provenance and valid taxonomy.

    Returns inventory counts, not classifier scores. An unreviewed dataset can
    validate structurally but is explicitly marked ineligible for training.
    """
    if not isinstance(records, list) or not records:
        raise ValueError("Dataset must be a nonempty list")
    ids, names, families = set(), set(), {}
    categories, policies, splits = Counter(), Counter(), Counter()
    coverage = defaultdict(set)
    reviewed = 0
    for index, row in enumerate(records, 1):
        def require(condition, message):
            if not condition:
                raise ValueError(f"Record {index}: {message}")
        require(isinstance(row, dict) and set(row) == FIELDS, "missing or unexpected fields")
        require(row["schema_version"] == "0.1.0", "unsupported schema")
        for field in ("id", "family_id", "filename", "file_type", "category", "expected_policy", "label_reason", "split"):
            require(isinstance(row[field], str) and bool(row[field].strip()), f"invalid {field}")
        require(row["id"] not in ids, "duplicate id")
        ids.add(row["id"])
        name = row["filename"]
        require(name not in {".", ".."} and not any(c in name for c in ("/", "\\", "\x00")), "filename must be a basename")
        normalized = name.casefold()
        require(normalized not in names, "duplicate filename (case-insensitive)")
        names.add(normalized)
        require(row["split"] in {"train", "validation", "test"}, "invalid split")
        family = row["family_id"]
        require(family not in families or families[family] == row["split"], "family leakage across splits")
        families[family] = row["split"]
        require(row["category"] in CATEGORIES, "invalid category")
        require(row["file_type"] in TYPES, "invalid file_type")
        require(row["expected_policy"] in POLICIES, "invalid expected_policy")
        if row["category"] == "Unknown":
            require(row["expected_policy"] != "suggest_category", "Unknown cannot suggest a destination")
        require(type(row["reviewed"]) is bool, "reviewed must be boolean")
        source = row["provenance"]
        require(isinstance(source, dict) and set(source) == {"kind", "source", "rights_status"}, "invalid provenance")
        require(all(isinstance(value, str) for value in source.values()), "provenance values must be strings")
        require(source["kind"] == "synthetic", "starter validator accepts synthetic data only")
        require(source["source"] == "filepilot-authored-starter", "unknown source")
        require(source["rights_status"] in {"pending_review", "approved"}, "invalid rights status")
        categories[row["category"]] += 1
        policies[row["expected_policy"]] += 1
        splits[row["split"]] += 1
        coverage[row["category"]].add(row["split"])
        reviewed += row["reviewed"]
    missing = {c: sorted({"train", "validation", "test"} - coverage[c])
               for c in sorted(CATEGORIES) if len(coverage[c]) != 3}
    return {"schema_version": "0.1.0", "record_count": len(records), "family_count": len(families),
            "categories": dict(sorted(categories.items())), "policies": dict(sorted(policies.items())),
            "splits": dict(sorted(splits.items())), "reviewed_count": reviewed,
            "training_eligible": not missing and reviewed == len(records) and all(r["provenance"]["rights_status"] == "approved" for r in records),
            "missing_category_split_coverage": missing}


def _unique_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON key " + key)
        result[key] = value
    return result


def load_records(path):
    records = []
    with Path(path).open(encoding="utf-8") as stream:
        for number, line in enumerate(stream, 1):
            if not line.strip():
                raise ValueError(f"Blank JSONL line {number}")
            try:
                records.append(json.loads(line, object_pairs_hook=_unique_keys))
            except ValueError as exc:
                raise ValueError(f"Invalid JSON on line {number}: {exc}") from exc
    return records


def main():
    parser = argparse.ArgumentParser(description="Validate explicit synthetic FilePilot dataset")
    parser.add_argument("dataset", help="JSONL path; no file discovery or scanning")
    args = parser.parse_args()
    try:
        result = validate_records(load_records(args.dataset))
    except (ValueError, OSError, UnicodeError) as exc:
        print(str(exc), file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
