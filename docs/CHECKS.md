# Project checks

The check command runs from the installed source location so it does not scan arbitrary personal folders

It returns a JSON report with separate statuses for each check

An unsuccessful check makes the command return exit code 1

## Core mode

```sh
PYTHONPATH=src python3 -m filepilot.check
```

Core mode runs the non ML tests validates both synthetic dataset snapshots checks expansion preserves the original rows validates the development review pool verifies recorded dataset hashes and report arithmetic and scans a temporary synthetic fixture through the CLI

Core mode does not fit a model even when optional dependencies are installed

Optional ML tests are reported as omitted rather than passed

The current core suite has 47 tests

A missing dataset malformed report failed test or empty test discovery is a failed check

Pending human labels and provisional rights status are recorded explicitly without making the structural checks fail

These structural checks do not establish label quality independent evaluation or deployment readiness

## Research mode

```sh
python3 -m pip install -r requirements-ml.txt
PYTHONPATH=src python3 -m filepilot.check --with-ml
```

This mode also runs the five optional ML tests and refits the baseline on synthetic training rows

It compares rules and ML on train and validation data and recomputes the fixed threshold diagnostic

The current research suite has 52 tests

No test split is scored no production threshold is selected and no model is saved

Missing optional dependencies cause an unsuccessful research check rather than a hidden pass

Package installation needs internet access but the research experiment itself does not make network requests

## Reading the report

The top level passed flag means all requested technical checks passed

It does not mean the product is ready for autonomous file operations

The review pool still reports 40 pending labels and no independent final test

Recorded reports are checked for dataset hashes and internal consistency in core mode

Fresh model results are only recomputed in research mode

The dataset review rights and research limitations stay visible in both modes
