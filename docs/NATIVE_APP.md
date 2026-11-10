# Native macOS alpha

## What the app implements

The desktop directory contains the React TypeScript frontend and Tauri application
The native core is a separate Rust crate with no webview or network dependency
A small Swift bridge handles system folder selection security scoped bookmarks login item registration and optional local English embeddings

The macOS folder picker is the only path used to grant a new root
Bookmarks and the selected directory identity are retained in the private local application database
Stale or unavailable bookmarks require the user to choose the folder again
Replacing the selected directory requires explicit renewal rather than authorizing a new directory at the old path
Revocation clears the bookmark while retaining operation references for Undo after renewed authorization
Overlapping roots are rejected
Full Disk Access Accessibility and Finder Automation are not requested
The development build is not App Store sandboxed and must not be described as sandbox audited

## First use

Start with disposable test folders
The setup wizard saves everyday preferences and offers automatic startup through the system login item
The user can choose multiple folders in one macOS panel
Each folder retains its own bookmark and directory identity
Only selected folders are covered and the app does not claim control over every Finder location

The first cleanup preview inventories the structure and lists source destination and reasoning
It also lists only the missing category folders needed by the current stable known proposals
Nothing is created during preview
Starting maintenance validates the preview token settings authorization epoch and a fresh inventory
Changed files or folder structure require a fresh preview
Accepted category creation is journaled before the root becomes eligible for background work
A mode or access change can cancel setup directory creation before the next change
Smart approval still requires an individual approval for every move

The overview provides Pause and Resume
Background inventories populate the review inbox without a manual scan
Preferences folder coverage duplicate tools personal rules and activity are separate pages
Changing settings invalidates pending proposals
A cancellation epoch is checked immediately before a move or Undo so a settings or access change can cancel work that is still hashing
Revoking access removes the root from active processing

Full autonomy is the initial preference
Smart approval never applies a proposal in the background
Limited autonomy uses explicitly authorized category destination rules
Renaming currently normalizes whitespace rather than inventing document titles
Unknown or ambiguous classifications require explicit review

## Filesystem boundaries

The engine opens authorized roots and parent directories without following symlinks
Projects generated folders common secret paths packages cloud placeholders hardlinks and special files are protected
Partially downloaded temporary files are excluded
File identity size timestamps and a SHA256 digest are verified around moves
Files must have a modification time at least ten seconds old
This age check and metadata comparison do not prove that another process has closed the file
Moves are limited to regular single link files at most 64 MiB
The planner inventories safe directories before choosing destinations
It can reuse exact named category folders or unambiguous aliases such as College Work and Photos
Multiple equally suitable existing destinations require review
Ordinary nested files are analysed recursively
Known files already matching a meaningful ancestor folder can stay without a proposal
Unknown or potentially misfiled nested files enter the review inbox
Nested relocation suggestions never gain automatic permission just from filename rules
This is a conservative folder name heuristic rather than a semantic understanding of every folder
Full autonomy may create a missing canonical category folder during ongoing maintenance after first setup
Smart and Limited modes do not create new folders in the background
Execution destinations must be existing folders inside the same authorized root
Cross volume moves are rejected rather than copied

The final rename uses kernel exclusive destination flags
No fallback that overwrites a destination is allowed
Directory handles and identity checks reduce races but do not make an adversarial changing filesystem an atomic snapshot
A directory renamed outside an authorized root while its handle remains open is a residual race requiring further security review
Use the alpha with disposable folders only until the Mac acceptance and security gates pass

## Journal Undo and crash recovery

The SQLite journal records an intent before a move rename Trash action or category directory setup
Journal writes use full synchronous durability
The engine records the resulting identity after exclusive rename and parent directory synchronization
Interrupted moves and Undo are reconciled on restart only when the expected identity and content digest match an unambiguous location
Ambiguous results are marked needs review
Unresolved journal operations block further moves in that root
Undo refuses changed files occupied original destinations and unauthorized roots

The history contains relative paths hashes and metadata rather than document contents
Category directory setup is journaled but is not automatically undone because removing directories needs a separate deletion policy
A storage device filesystem or process failure can exceed the durability guarantees of these primitives

## Background processing

The notify recommended backend uses macOS FSEvents on macOS
Events trigger reconciliation and a thirty second sweep covers missed events
The event queue is bounded and events are coalesced
Unchanged preview identifiers survive reconciliation
A manually selected destination remains manual until explicitly approved
Changed source identities expire earlier preview actions
The closed window is hidden while the app continues running
Click the Dock icon to reopen it
Quit stops the process
Login item registration uses SMAppService and can require approval in System Settings

Sleep pauses local execution
The running process reconciles after wake
Restart requires a functioning login item and bookmark resolution
No execution occurs while the computer is shut down
Real sleep lock restart and login behaviour has not been accepted on a user Mac yet

## Duplicate analysis

Duplicate detection is a separate explicit read only operation
It hashes candidate files with matching sizes and reports matching SHA256 groups
Files over 64 MiB are excluded and each request hashes at most 256 MiB
Limits and errors can make the result partial
Duplicate groups are observations rather than deletion permission
Duplicate candidates can be selected for a separately confirmed Trash action
Permanent deletion is unavailable

## Separately confirmed Trash actions

The app shows a native Cancel or Move to Trash confirmation for each requested file
Full autonomy never skips this confirmation
The trusted Swift bridge obtains the system Trash location rather than accepting it from an imported document
The Rust engine verifies source identity Trash directory ownership and same volume placement before exclusive rename
The Trash operation is journaled and its current identity and content digest are checked during Undo
A mode change root renewal or revoked access invalidates an earlier Trash confirmation
Trash names carry a unique FilePilot prefix and the journal retains the original name
Finder Put Back metadata is not created by this implementation so restore through FilePilot Undo
An occupied original location a changed trashed file or a replaced Trash directory blocks Undo
System Trash access must still be accepted on a real Mac
No Full Disk Access bypass is attempted if macOS denies access

## Local semantic inspection

Inspect locally reads a selected pending text file only after the user requests inspection
Supported content is UTF8 text markdown CSV and logs up to 64 KiB
Only a bounded two thousand character sample is sent to the local framework
PDF files up to two MiB are inspected in a separate bundled PDFKit process
The worker receives a bounded byte snapshot rather than a document path
It inspects at most five pages and returns at most two thousand characters
A four second CPU limit and five second parent timeout apply
Timeout terminates the parser process group
The process boundary is not an operating system sandbox and does not enforce a reliable macOS memory ceiling
Encrypted malformed and image only documents are skipped or return no text
OCR is not implemented
Other file types use filenames only
Document strings are input data and are never executed as commands

The Swift bridge uses the installed Apple English sentence embedding when available
It makes no asset download request and has an explicit unavailable fallback
The semantic result contains advisory category rankings rather than calibrated probabilities
Choosing a semantic suggestion still requires a separate explicit move approval
The automatic organizer does not execute semantic suggestions
No independent accuracy claim is made
The custom classifier remains a research module pending human labels and evaluation

## Personalization

Explicit category destination corrections are stored per authorized root
They only affect future proposals within that root
Disabling learning prevents applying or recording corrections
Clearing learning removes logical correction records
Approved limited mode rules have a separate clear control
Remember this destination stores an explicit filename example within the authorized root
Nearest filename token matches can suggest future destinations but always require review
Matching only a file extension does not transfer a correction
Explicit category preferences and limited mode rules take precedence over nearest examples
The current lookup is bounded to one thousand examples per root
This is local preference and nearest neighbor learning rather than neural retraining
No data is uploaded or shared with other users

## Development builds

The native workflow builds a universal macOS application for Apple silicon and Intel and archives it with executable permissions preserved
Build artifacts are temporary development downloads rather than a signed public release
Signing notarization update delivery and release security review remain pending
No Apple developer credentials should be committed to this repository

## Sources

https://tauri.app/develop/calling-rust/
https://tauri.app/security/capabilities/
https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox
https://developer.apple.com/documentation/servicemanagement/smappservice
https://developer.apple.com/documentation/naturallanguage/nlembedding
https://doc.rust-lang.org/std/fs/fn.rename.html

https://developer.apple.com/documentation/pdfkit/pdfdocument
https://developer.apple.com/documentation/foundation/filemanager/searchpathdirectory

See SETUP_WORKFLOW for the one time setup contract and current structure planning limits

## Important collections and unwanted files

Collections lists ordinary files in the latest safe inventory of an approved folder
The user explicitly selects what is important or unwanted
Selected files can be added to an existing relative folder or to a new named folder at the authorized root
An empty title receives a filename based title suggestion during preview
The user can edit that title before generating another preview
Creating and filling a collection requires its own confirmation
No file content is read to infer importance
No file is marked unwanted automatically

A collection supports at most one hundred files and two hundred fifty six MiB per operation
The existing per file sixty four MiB stability project path link and collision restrictions still apply
The whole selected group is preflighted before the first change
External races or permission changes can still interrupt a group after some moves
Each completed move has its own journal record and Undo
The result reports completed moves and any error rather than claiming an atomic transaction
An empty directory can remain after Undo because automatic directory deletion is not authorized

Move selected to Trash asks for separate native confirmation for each file
Cancelling stops the remaining requests
Use Activity to undo confirmed Trash actions
Permanent deletion and importing from outside the selected authorized root are not implemented
The user can grant additional folders separately

## Summaries and review decisions

Summarise reads supported content locally and returns a short extractive overview of sampled passages
It does not use a generative model or claim full semantic comprehension
UTF8 text Markdown CSV and log files are limited to sixty four KiB input and two thousand returned characters
PDFs use the existing isolated trusted helper with a two MiB input limit five pages and two thousand characters
PDF input is snapshot checked again after parsing
Truncation unavailable text and unsupported formats are explicit
There is no OCR image understanding archive extraction or Office document parser in this iteration

Related folder recommendations combine content keyword topics existing folder names filename overlap and at most eight neighbouring text samples
Each neighbour is at most sixty four KiB and two thousand characters
All neighbours remain inside the same authorized root and pass the existing protected project link and placeholder checks
Suggestions are heuristics and never grant automatic move permission
A displayed folder can be selected explicitly for a guarded reversible move
Create a titled collection opens Collections with the source and suggested title already filled in
Keep here saves the source identity locally suppressing unchanged future proposals until the file changes or the user clears those decisions
A manually reviewed destination also records that explicit placement using the post move identity in the same transaction as the applied journal state
This prevents a newly named collection from being immediately flagged again
Keep here is separate from personalization and clearing learning does not silently discard that explicit choice

## Review notifications

Review notifications are off by default and require an explicit preference and system permission request
They use the local UserNotifications framework rather than a remote push service
Messages contain counts rather than filenames paths or document contents
Notifications are batched with a fifteen minute cooldown and a newly pending item check
macOS permission and installed app identity determine actual delivery
Real permission prompts click activation delivery and sleep wake acceptance remain unchecked until tested on a Mac

https://developer.apple.com/documentation/usernotifications/unusernotificationcenter/requestauthorization(options:completionhandler:)
