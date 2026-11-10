# Development roadmap

I already have a local prototype and I am bringing it into this repository one part at a time

These steps show what has been added here rather than when the local work was originally done

## First steps

- [x] Add the project overview and ignore local development files
- [x] Add the Python package and read only scanner
- [x] Add project detection and filesystem safety checks
- [x] Add synthetic test fixtures and automated tests
- [x] Add category rules and the JSON report format
- [x] Add dataset validation and checks for split leakage
- [x] Add the local ML baseline and its evaluation results
- [x] Record the experiments with uncertain predictions
- [x] Add one command to run project checks
- [x] Run the checks automatically on GitHub

## After the foundation

- Build a more varied dataset and review the labels
- Explore local embeddings and folder recommendations
- Build the macOS interface with Tauri and React
- Add a Rust engine for file operations
- Keep permission checks separate from model predictions
- Add autonomy modes an operation history and Undo
- Add folder permissions and background monitoring on macOS
- Learn from explicit user corrections on the device
- Test recovery security and packaging before a release

## What has been checked so far

The scanner has 21 focused tests

The project check records its current test counts in the JSON report

Both modes have passed in the development workspace

The Actions tab records the checks run on GitHub

The ML experiments have only used small synthetic datasets so far

I have looked at validation errors while improving the training data so those results are development feedback rather than an independent accuracy benchmark

There is no production threshold for automatic classification yet


## Work that is still pending

The foundation checklist records repository capabilities rather than a finished macOS product

Independent human labels and a separately authored final evaluation set are still needed

Embeddings folder ranking the desktop interface filesystem operations Undo monitoring and personalization remain later milestones

The scanner stays read only until the next milestone is designed and tested


## Document inspection milestone

- [x] Add explicit local text extraction with bounded input and output
- [x] Keep extracted text out of reports unless requested
- [x] Test path boundaries file changes and untrusted text
- [x] Add optional PDF extraction in a separate bounded worker
- [x] Run document checks on GitHub with the optional parser installed

This milestone extracts text rather than claiming semantic understanding or automatic organisation

## Destination and review foundations

- [x] Add deterministic destination ranking and metric helpers
- [x] Generate read only collision aware plans
- [x] Separate policy decisions from ranking
- [x] Store explicit scoped corrections locally
- [x] Add disable and clear controls to the preference component
- [x] Add an offline browser review prototype

See IMPLEMENTATION_STATUS for the native product work still pending

## Native alpha

- [x] Add the React and Tauri application
- [x] Add the Rust move rename journal and Undo implementation
- [x] Wire root authorization autonomy revisions corrections and rules
- [x] Add the Swift bookmark login and local embedding bridges
- [x] Add FSEvents backed monitoring and scheduled reconciliation
- [x] Add explicit read only duplicate analysis
- [x] Add native core tests and macOS app build checks

Production acceptance remains tracked in IMPLEMENTATION_STATUS and MAC_ACCEPTANCE
Earlier pending lists describe the roadmap at the time and are superseded by the current implementation status

## Native document and recovery workflows

- [x] Add a bundled PDF inspection process with bounded input and output
- [x] Add explicit native Trash confirmation in every autonomy mode
- [x] Journal Trash moves and protect Trash Undo
- [x] Record category directory setup and recover interrupted setup
- [x] Save explicit filename examples for conservative local suggestions
- [x] Keep nearest example suggestions behind manual review
- [x] Honor custom destinations in Limited autonomy

The production release gates remain in IMPLEMENTATION_STATUS

## One time setup iteration

- [x] Replace repeated manual scan workflow with setup and background review inbox
- [x] Allow multiple explicit folder grants in the macOS picker
- [x] Persist whether existing named folders should be reused
- [x] Inventory safe folder structure before planning loose file moves
- [x] Keep existing collections together rather than flattening them
- [x] Preview needed new folders and validate the plan again before setup
- [x] Offer startup once during onboarding
- [x] Add synthetic workflow regression tests
- [ ] Complete real Mac folder permission startup and sleep wake acceptance
- [ ] Evaluate broader semantic structure recommendations with reviewed data
- [ ] Sign notarize and independently audit a production release

## Important collections

- [x] Select safe files from approved folders
- [x] Add selected files to an existing folder
- [x] Preview a new named collection for important files
- [x] Suggest an editable title using filenames
- [x] Confirm unwanted files through system Trash with Undo
- [x] Preflight grouped moves and report partial completion honestly
- [x] Test collections with synthetic data
- [ ] Validate the complete collection and Trash flow on a real Mac

## Recursive assisted review

- [x] Analyse ordinary nested files instead of skipping their folders
- [x] Preserve detected coding projects and protect common markerless source layouts
- [x] Show sampled local text and PDF overviews
- [x] Suggest related existing folders using bounded local evidence
- [x] Keep files in place with an explicit persistent decision
- [x] Seed a titled collection from a summary
- [x] Implement optional local review notifications
- [x] Create a more realistic deeply nested synthetic test pack
- [ ] Verify notification consent delivery activation and lifecycle on a real installed Mac app
- [ ] Add audited support for more document formats and OCR
- [ ] Evaluate richer local summarization models with licensed reviewed data

## Main locations update

See [workspace behavior and limits](WORKSPACE.md) for approved routing cross location transfers and the two item focus
The core suite has 115 passing synthetic tests on Linux and macOS
All 31 React browser workflows pass against the production frontend and synthetic native bridge
Real Mac acceptance must check both grants folder picker renewals route previews per location counts file and folder transfer Undo and sleep wake reconciliation
Use disposable sibling test roots for these checks
