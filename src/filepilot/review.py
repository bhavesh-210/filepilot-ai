"""Validate the unlabeled development review pool without generating predictions"""
import argparse
from collections import Counter
import json
from .dataset import load_records, TYPES
from .report import CATEGORIES

FIELDS = {"id", "family_id", "filename", "file_type", "review_label", "reviewer",
          "review_kind", "evidence_sufficient", "review_note", "purpose", "provenance"}


def validate_review_pool(rows):
    if not isinstance(rows, list) or not rows:
        raise ValueError("Expected a nonempty review pool")
    identifiers, names, families = set(), set(), set()
    complete = human = 0
    categories = Counter()
    for number, row in enumerate(rows, 1):
        def require(condition, message):
            if not condition:
                raise ValueError(f"Review row {number}: {message}")
        require(isinstance(row, dict) and set(row) == FIELDS, "missing or unexpected fields")
        for field in ("id", "family_id", "filename", "file_type", "purpose", "provenance"):
            require(isinstance(row[field], str) and bool(row[field].strip()), "invalid " + field)
        require(row["id"] not in identifiers, "duplicate id")
        identifiers.add(row["id"])
        name = row["filename"]
        require(name not in {".", ".."} and not any(c in name for c in ("/", "\\", "\x00")), "filename must be a basename")
        require(name.casefold() not in names, "duplicate filename")
        names.add(name.casefold())
        families.add(row["family_id"])
        require(row["file_type"] in TYPES, "invalid file type")
        require(row["purpose"] == "development_review", "not a final test set")
        require(row["provenance"] == "AI-authored synthetic filename; no document contents", "unknown provenance")
        require(isinstance(row["review_note"], str), "review note must be text")
        if row["review_label"] is None:
            require(row["reviewer"] is None and row["review_kind"] is None and row["evidence_sufficient"] is None,
                    "unlabeled rows must not claim a completed review")
        else:
            require(isinstance(row["review_label"], str) and row["review_label"] in CATEGORIES, "invalid label")
            require(isinstance(row["reviewer"], str) and bool(row["reviewer"].strip()), "reviewer required")
            require(isinstance(row["review_kind"], str) and row["review_kind"] in {"human", "ai"}, "review kind required")
            require(type(row["evidence_sufficient"]) is bool, "evidence judgment required")
            require(bool(row["review_note"].strip()), "review rationale required")
            complete += 1
            human += row["review_kind"] == "human"
            categories[row["review_label"]] += 1
    return dict(record_count=len(rows), family_count=len(families), complete_count=complete,
                human_reviewed_count=human, pending_count=len(rows) - complete,
                ready_for_development_scoring=complete == len(rows),
                all_labels_human_reviewed=human == len(rows), independent_final_test=False,
                reviewed_categories=dict(categories))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pool")
    args = parser.parse_args()
    try:
        print(json.dumps(validate_review_pool(load_records(args.pool)), indent=2))
    except (OSError, ValueError, UnicodeError) as exc:
        parser.exit(2, str(exc) + "\n")


if __name__ == "__main__":
    main()
