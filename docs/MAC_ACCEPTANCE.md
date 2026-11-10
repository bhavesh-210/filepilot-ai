# Mac acceptance checklist

Use a disposable local test folder with a separate backup
Do not use Desktop Documents Downloads or an iCloud folder for the first acceptance run
These boxes must only be checked after a person verifies the installed app

- [ ] App opens through normal macOS installation security mechanisms
- [ ] User can cancel folder selection without authorizing any root
- [ ] Only the selected folder is scanned
- [ ] Initial preview does not move any file before acknowledgement or individual approval
- [ ] Full autonomy moves a newly added stable synthetic file after preview acknowledgement
- [ ] Smart approval records proposals without background moves
- [ ] Limited autonomy only follows a deliberately approved rule
- [ ] A mode change cancels pending execution permissions
- [ ] Rename off keeps original filenames
- [ ] Project directories symlinks hardlinks and secret paths stay protected
- [ ] Destination collisions preserve both existing and incoming files
- [ ] Undo restores a moved file and refuses changed or occupied destinations
- [ ] Restart recovers a deliberately interrupted operation conservatively
- [ ] Revoked roots stop being processed
- [ ] Stale bookmarks require renewed folder selection
- [ ] Learning disabled records and applies no correction
- [ ] Clear learning and clear rules have separate effects
- [ ] Duplicate analysis makes no file changes
- [ ] Semantic inspection has an explicit unavailable fallback and sends no document over the network
- [ ] Window close keeps monitoring active and Dock click restores the window
- [ ] Explicit Quit stops background processing
- [ ] Login item registration matches the state shown by System Settings
- [ ] Screen lock permits processing while the Mac remains awake
- [ ] Sleep pauses work and wake reconciles missed changes
- [ ] Shutdown runs no local work and login resumes only after authorization restoration

Cloud provider tests and release signing must be tracked separately before using personal folders
Do not remove macOS quarantine attributes or weaken system security to make acceptance pass
