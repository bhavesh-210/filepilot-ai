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
- [ ] Record the experiments with uncertain predictions
- [ ] Add one command to run project checks
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

The local prototype has passed 31 automated tests on my Mac

The scanner change includes 21 focused tests which have passed in the development workspace

The larger local suite also covers experiments that have not been added here yet

The ML experiments have only used small synthetic datasets so far

I have looked at validation errors while improving the training data so those results are development feedback rather than an independent accuracy benchmark

There is no production threshold for automatic classification yet
