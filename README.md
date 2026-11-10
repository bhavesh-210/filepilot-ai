# FilePilot AI

Your files your Mac your control

I am building FilePilot AI to make it easier to keep files organised on a Mac

The idea is to understand what a file belongs to and help put it in the right place while keeping the user in control

## Where things stand

I have a working read only Python prototype that scans a selected test folder and suggests categories without changing any files

It reads basic metadata recognises common file types and spots coding projects so their structure can be preserved

I have also started testing a small local ML classifier using synthetic examples

I am adding this existing work to the repository in small steps

This repository now includes the scanner and its automated tests

The repository also includes synthetic datasets and local ML experiments

The experiments remain separate from the scanner

## What I want to build

- File analysis that works locally without cloud AI APIs
- A preview before the first cleanup
- Support for keeping coding projects together
- Different levels of control over automatic organisation
- A history of changes with an option to undo them
- Background monitoring for newly added files

Moving files to Trash or deleting them will always need separate confirmation

## How I am approaching it

I am starting with a read only scanner before building anything that moves or renames files

The current experiments use synthetic examples rather than personal documents

Privacy and reliability matter more to me than getting a quick demo working

The development roadmap is in the docs folder


## Trying the scanner

You will need Python 3 point 11 or newer on macOS or Linux

The scanner uses the Python standard library and does not need extra runtime packages

Run the checks from the project folder

```sh
PYTHONPATH=src python3 -m unittest discover -s tests -v
```

For a scan provide a test directory you choose

```sh
PYTHONPATH=src python3 -m filepilot.cli /absolute/path/to/your/test-folder
```

The report is printed as JSON and nothing is moved renamed or deleted

There is no default directory and discovered files are never executed

## Limits to keep in mind

File types and categories are suggestions based on names and extensions

Document contents are not read and duplicates are not checked

Symlinks are not followed and roots containing symlink components are rejected

The inventory is not an atomic snapshot so a folder changing during a scan can produce a partial result

The scanner limits the number of reported entries and traversal depth but still lists each directory in memory

Sensitive folder exclusions are conservative rules rather than a complete detector

Use synthetic files in a local test folder while this is a prototype

Cloud folder providers such as iCloud may react to metadata access so this version does not promise offline behaviour for provider managed folders

Reports contain file paths so keep them private

See docs/SCANNER.md for the report fields and safety boundaries


## Automatic checks

GitHub runs core project checks after each push to main

The core checks cover Python 3 point 12 and 3 point 14 on Linux and macOS

A separate Linux job installs the optional research dependencies and runs fresh ML diagnostics

Open the Actions tab to see results for a commit and the JSON reports saved with each run


## One command for project checks

GitHub can run the same project checks without asking you to copy commands

The core check uses only the standard library and does not fit a model

```sh
PYTHONPATH=src python3 -m filepilot.check
```

Optional research checks also fit the local baseline and measure uncertain predictions

```sh
PYTHONPATH=src python3 -m filepilot.check --with-ml
```

The command prints a JSON report and exits unsuccessfully if a check fails

Pending human labels are reported as pending rather than counted as reviewed

The research reports and their limits are documented in the docs folder


## Inspecting document contents

There is a separate opt in inspector for a single explicitly selected local document

The metadata scanner stays unchanged and extracted text is hidden from reports unless requested

The current text formats are text Markdown CSV and log files

See docs/CONTENT.md for the supported formats limits and safety boundaries

PDF inspection is a separate opt in with an optional pinned local parser
Document checks run automatically on Linux and macOS
Encrypted documents are skipped and OCR is not included
The desktop interface and semantic folder recommendations are later milestones

## Destination and review foundations

I can now create read only plans from scanner reports and explicitly chosen destination folders
The baseline combines category evidence and lexical overlap without a downloaded model
The separate policy component covers three autonomy modes but no executor is connected
Explicit local corrections can be disabled or cleared

The offline review prototype is in frontend index html
It can show a synthetic demo import a plan and export review choices
It never changes files and does not replace the planned native macOS app

See [implementation status](docs/IMPLEMENTATION_STATUS.md) and [planning details](docs/PLANNING.md)
