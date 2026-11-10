# Read only scanner

## What it does

Scans only an explicitly provided root and records relative paths file kinds sizes and modification times

Identifies common extensions and project markers including package manifests and Git folders

Project contents stay together with a leave untouched recommendation

Other classifications are advisory and require review

## Safety boundaries

Directory handles use no follow opens and compare directory identity before descent

Symlinks special files application bundles and selected sensitive or generated folders are left untouched

The scanner does not read file contents execute discovered code or make file changes

There is no network client in this package

A directory can still be renamed after its handle has been opened so the report must never be used as execution permission

Concurrent changes can produce a partial inventory and exclusions are not a complete sensitive file detector

## Report contract

The report schema is version 0 point 1 point 0

The top level records the scanner version root timestamp limits entries detected projects errors and summary counts

Each entry records its relative path file kind extension type size modification time project root status and classification

Classification records the category reason suggested category review flag and advisory recommendation

The validator checks report structure project membership review requirements and summary consistency

## CLI results

Exit code 0 means a complete scan

Exit code 1 means errors were recorded or a configured limit was reached

Exit code 2 means the root or arguments were unavailable or invalid

JSON goes to standard output and root errors go to standard error

If redirecting a report keep its output path outside the scan root

## Test coverage

The 21 tests use temporary synthetic files and include inaccessible directories symlinked paths project boundaries special files classification conflicts report validation and the command line entry point

The temporary root is resolved before scanning to support macOS system paths such as the alias from var to private var
