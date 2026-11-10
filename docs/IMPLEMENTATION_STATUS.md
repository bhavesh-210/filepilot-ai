# Implementation status

## Tested capabilities

- [x] Metadata scanner with explicit roots
- [x] Rule based categories and project preservation
- [x] Local bounded text and optional PDF extraction
- [x] Synthetic dataset validation and experimental classifier
- [x] Destination ranking baseline and metric helpers
- [x] Read only organization plan with collision checks
- [x] Separate autonomy policy logic
- [x] Local explicit correction storage with disable and clear
- [x] Offline browser review prototype

The policy and preference components are tested independently but are not wired into a native desktop app
The browser review is advisory and does not approve filesystem execution

## Pending product work

- [ ] Independent human labels and destination benchmark
- [ ] Compact local semantic model and measured model quality
- [ ] React and Tauri native macOS interface
- [ ] System folder picker and persistent security scoped bookmarks
- [ ] Rust move and rename executor with no overwrite guarantees
- [ ] Durable operation journal crash recovery and verified Undo
- [ ] Rename quality evaluation and explicit rename controls in the native app
- [ ] iCloud provider state handling in the native backend
- [ ] FSEvents monitor and scheduled reconciliation
- [ ] Login item registration and wake restart reconciliation
- [ ] Wire policy settings and revisions into every pending native action
- [ ] Native personalization controls and scoped correction integration
- [ ] Nearest neighbor personalization and retraining evaluation
- [ ] Security audit signed notarized build and release validation

## Current environment boundary

The development workspace runs Linux and has no installed Rust or Swift toolchain
GitHub jobs verify the Python components on Linux and macOS
Native permission dialogs background behavior and release packaging require a built app and macOS acceptance tests
The prototype is not a production autonomous organizer

Awake locked and window closed operation require a registered background agent
Sleep pauses processing and wake must reconcile changes
No software runs locally while the Mac is shut down
