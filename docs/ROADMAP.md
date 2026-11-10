# Development roadmap

I already have a local prototype and I am bringing it into this repository one part at a time

These steps show what has been added here rather than when the local work was originally done

## First steps

- [x] Add the project overview and ignore local development files
- [ ] Add the Python package and read only scanner
- [ ] Add project detection and filesystem safety checks
- [ ] Add synthetic test fixtures and automated tests
- [ ] Add category rules and the JSON report format
- [ ] Add dataset validation and checks for split leakage
- [ ] Add the local ML baseline and its evaluation results
- [ ] Record the experiments with uncertain predictions
- [ ] Add one command to run project checks
- [ ] Run the checks automatically on GitHub

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

Those checks will need to pass again against the code added to this repository

The ML experiments have only used small synthetic datasets so far

I have looked at validation errors while improving the training data so those results are development feedback rather than an independent accuracy benchmark

There is no production threshold for automatic classification yet
