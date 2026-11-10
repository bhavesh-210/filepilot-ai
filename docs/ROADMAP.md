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
