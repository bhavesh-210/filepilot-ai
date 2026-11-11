# Implementation status

## One time setup iteration

The native app now has onboarding overview automatic review inbox folder coverage activity and preferences
Folder permissions are granted explicitly through one multi selection system picker
The first cleanup plan includes safe structure inventory and needed folder creation
Setup completion requires an unchanged preview and current permissions
The planner preserves existing collections and reuses named folders for loose files
This is conservative metadata planning and not a validated model that can redesign every Finder folder
Automatic duplicate removal and permanent deletion are not implemented
Real Mac permission and lifecycle acceptance and production signing remain open

## Important file collections

The UI now supports explicit file selection editable filename based folder title suggestions new collection previews and moves into existing folders
The user can select unwanted ordinary files for separately confirmed Trash
The implementation does not autonomously infer which files are important or safe to discard
Group moves use per file journaling and can stop with partial completion when an external change is detected
Cross volume transfers permanent deletion and real Mac acceptance remain outside this iteration

## Recursive analysis and sampled summaries

Ordinary subfolders are now analysed recursively while manifest projects common source layouts and protected paths are skipped
Review includes a local extractive content overview persistent Keep here related existing folder moves and a shortcut to a preselected titled collection
Text and PDF summaries are bounded samples and unsupported formats are reported honestly
No large language model OCR general Office parser or claim of full semantic understanding is included
Optional local notification code is compiled and system delivery remains a real Mac acceptance gate

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
- [x] Explicit corrections and filename examples with disable and clear controls
- [x] Conservative nearest neighbor suggestions that always require review
- [x] FSEvents backend and scheduled reconciliation implementation
- [x] Window closed monitoring and login item bridge
- [x] Read only bounded duplicate detection
- [x] Optional installed macOS English semantic suggestions with manual review
- [x] Bundled isolated PDF inspection with input page text and time limits
- [x] Separately confirmed Trash and guarded Trash Undo
- [x] Category directory setup journal and migration support

Implementation does not mean production acceptance
The native alpha needs the release gates below
See NATIVE_APP for exact limits and residual risks

- [x] Patched bundled SQLite and automated dependency advisory checks

## Release gates still open

- [ ] Test system permissions and stale bookmark renewal on a real user Mac
- [ ] Test lock sleep wake restart login item and window lifecycle on a real Mac
- [ ] Validate iCloud and other File Provider behaviour without accidental hydration
- [ ] Security review filesystem races parser inputs webview permissions and recovery
- [x] Synthetic database failure injection before moves after moves and during Undo
- [ ] Storage device and provider failure injection beyond database trigger simulations
- [ ] Human label review and an independent destination benchmark
- [ ] Evaluate semantic quality language coverage and unknown handling
- [ ] Validate native PDF parsing against a broader licensed adversarial corpus
- [ ] Evaluate personalization quality and collect enough explicit corrections before neural retraining
- [ ] Accept native Trash confirmation system permissions and recovery on a real Mac
- [ ] Review directory setup recovery under storage failure
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

## Main locations update

See [workspace behavior and limits](WORKSPACE.md) for approved routing cross location transfers and the two item focus
The core suite has 115 passing synthetic tests on Linux and macOS
All 31 React browser workflows pass against the production frontend and synthetic native bridge
Real Mac acceptance must check both grants folder picker renewals route previews per location counts file and folder transfer Undo and sleep wake reconciliation
Use disposable sibling test roots for these checks

## Unified cleanup preset build

Implemented combined editable cleanup previews file kind and year presets optional UTC modification date prefixes saved preset automation and persistent cleanup run Undo
The UI now uses a restrained light system style with compact controls and a file table
The preview supports up to one hundred candidate files from up to sixteen selected roots
Destinations in this workflow stay inside each source root and existing cross root transfers remain separate
Unsupported kinds stay unselected by default
Created directories are retained when undoing a cleanup run
Content based vendor or subject naming natural language planning OCR and general semantic folder generation remain unimplemented
See CLEANUP for exact behavior
