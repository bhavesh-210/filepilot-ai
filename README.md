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

The ML experiments and datasets will follow in separate changes

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

GitHub runs the scanner tests after each push to main

The checks cover Python 3 point 12 and 3 point 14 on Linux and macOS

Open the Actions tab to see the results for a commit
