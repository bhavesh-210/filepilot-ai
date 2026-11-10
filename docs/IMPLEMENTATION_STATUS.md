# Implementation status

## Implemented foundations

- [x] Read only Python scanner report and classification rules
- [x] Bounded local text and optional Python PDF extraction
- [x] Synthetic dataset validation and experimental classifier
- [x] Local destination planning policy and metric helpers
- [x] React TypeScript frontend with Tauri native backend
- [x] System folder selection and persisted bookmark bridge
- [x] Rust exclusive moves and whitespace renaming
- [x] Persistent intent journal identity and digest checks
- [x] Conservative Undo and interrupted operation reconciliation
- [x] Three autonomy modes and stale proposal invalidation
- [x] Initial preview gate and authorized root revocation
- [x] Explicit corrections with disable and clear controls
- [x] FSEvents backend and scheduled reconciliation implementation
- [x] Window closed monitoring and login item bridge
- [x] Read only bounded duplicate detection
- [x] Optional installed macOS English semantic suggestions with manual review

Implementation does not mean production acceptance
The native alpha needs the release gates below
See NATIVE_APP for exact limits and residual risks

## Release gates still open

- [ ] Test system permissions and stale bookmark renewal on a real user Mac
- [ ] Test lock sleep wake restart login item and window lifecycle on a real Mac
- [ ] Validate iCloud and other File Provider behaviour without accidental hydration
- [ ] Security review filesystem races parser inputs webview permissions and recovery
- [ ] Failure injection across journal writes filesystem operations and storage faults
- [ ] Human label review and an independent destination benchmark
- [ ] Evaluate semantic quality language coverage and unknown handling
- [ ] Integrate and validate PDF content analysis in the native app
- [ ] Evaluate neural and nearest neighbor personalization before enabling it
- [ ] Implement separately confirmed Trash operations with verified Undo
- [ ] Record category directory creation in the operation journal
- [ ] Developer ID signing notarization and trusted installation validation
- [ ] Decide and test an optional transparent update mechanism

Permanent deletion remains unavailable
Cross volume moves large file execution and semantic title generation are outside the alpha
The app is not a completed production release

## Validation boundaries

Rust tests use disposable synthetic folders on Linux and macOS
React browser tests use a synthetic IPC mock and do not validate actual OS permission dialogs
A compiled app bundle is not evidence that user interaction lifecycle and all filesystem providers work correctly
The developer workspace has a locally installed isolated Rust toolchain
Swift and app bundling are compiled in the macOS GitHub runner

Signing credentials user Mac permission interaction and independent human judgments cannot be fabricated by automated development
