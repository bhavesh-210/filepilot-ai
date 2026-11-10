# Synthetic datasets

The repository includes authored metadata examples rather than real documents

No personal files or external datasets are included

## Current snapshots

- The starter set has 102 records in 54 declared families
- The expanded set has 134 records in 86 declared families
- Expansion adds 32 training examples and preserves every original record and split

Each record includes a filename file type category expected policy family split label reason and provenance

Draft and final variants from one family stay in the same split

The validator checks required fields category names duplicate identifiers duplicate filenames provenance and family leakage

It also rejects duplicate JSON keys and reports missing category coverage in each split

File names inside the dataset are data and are never opened or executed

## Checking a snapshot

```sh
PYTHONPATH=src python3 -m filepilot.dataset ai/datasets/starter-v0.1.0/records.jsonl
```

A successful structural check does not establish that the labels are correct

The current labels and rights status remain provisional so training eligibility is false

Internal research can use these authored synthetic examples through an explicit research only option

That option does not approve the labels or change their rights status

## Limits

Declared family checks cannot establish semantic independence

The examples contain repeated vocabulary and templates

The expanded training set was created after inspecting earlier validation errors

Repeated validation scores are development feedback rather than independent accuracy evidence

A broader separately reviewed dataset is still needed before choosing a production model
