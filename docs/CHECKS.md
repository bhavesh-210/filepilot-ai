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

The JSON report records how many core tests ran

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

The JSON report records how many tests ran with research dependencies

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


## GitHub automation

Each push to main runs four core jobs across Linux macOS and two Python versions

A separate Linux job installs the optional research dependencies and runs the full research mode

The reports are visible in job logs and are also saved as artifacts for seven days

The workflow uses read only repository permissions and pinned Node 24 actions

The shell keeps pipeline failures visible even when the report is copied through tee

An unsuccessful project check fails its GitHub job

The research job validates the experimental pipeline rather than a production model

## Document checks

The document jobs install the pinned optional parser on Linux and macOS
They run the core tests and synthetic PDF extraction tests
Core jobs omit PDF extraction tests and require no third party packages
PDF guard tests run even without the parser
Reports distinguish requested optional tests from omitted tests

## Planning and browser checks

The core suite includes deterministic planner policy and SQLite preference tests
A separate Chromium job verifies the offline review screen with synthetic plans
Browser tests check mode changes export boundaries invalid imports markup safety and narrow layouts
Browser tools are test only dependencies and are not used by the scanner

## Native application checks

A separate workflow tests the Rust core on Linux and macOS
The filesystem tests use disposable folders only
Fault tests simulate journal insert failure commit failure interrupted Undo and ambiguous recovery
Other tests cover root replacement permission cancellation revocation renewal and kernel exclusive collisions

The desktop job compiles the Swift helpers and a universal Tauri app
It preserves bundle permissions in a development artifact
The React workflow tests use a synthetic IPC mock
Real system dialogs login item approval cloud providers and sleep wake behaviour remain manual acceptance gates

The native core suite also tests the parser protocol timeout process group termination separately confirmed Trash Trash Undo and legacy journal migration
The Mac build job runs five synthetic PDFKit worker tests before bundling the helper
The React workflow covers cancelling Trash without approving an ordinary move
No automated test accepts a real system permission dialog or uses a personal document

The packaged PDF helper is tested again from inside the built app bundle to catch missing resources and executable permission errors
Remembering a destination is tested separately from approving a move

## Dependency checks

The dependency workflow checks both Rust lockfiles and the frontend lockfile
The bundled SQLite version has a security regression test
All platform informational advisories remain visible in the audit reports
See DEPENDENCY_AUDIT for the current results and checked Mac target scope
