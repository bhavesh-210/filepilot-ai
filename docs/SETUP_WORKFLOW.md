# One time setup and everyday maintenance

## User flow

Open Overview and choose everyday preferences
The initial settings use Full autonomy background monitoring filename whitespace cleanup explicit learning and separately confirmed Trash proposals
Existing named destinations are preferred by default
Automatic startup is offered during setup and still depends on macOS approval

Choose one or several folders in the macOS system picker
The app saves a bookmark and directory identity for each successful grant
A rejected folder is reported without discarding the other successful grants
Overlapping parent and child grants are rejected
Finder is a view of many locations rather than a single permission scope
Only the locations explicitly granted to FilePilot are covered

Understand my folders generates a read only first cleanup plan
The plan shows the safe folder structure file proposals protected entry count and only the missing category directories currently needed
Start maintaining my folders confirms that exact plan
Changed sources folder structure settings or authorization invalidate the plan and require a fresh preview
If one folder setup succeeds and another fails the successful folder remains marked as complete
Refresh the remaining preview before continuing

## What happens automatically

The existing event watcher and scheduled reconciliation generate current inventories
The review inbox reads live pending actions from the engine rather than requiring a manual scan
Applied expired and revoked proposals are removed from that inbox
No content analysis or confidence is fabricated by the dashboard

Full autonomy can create a needed canonical category directory after setup and execute stable unambiguous permitted proposals
Smart approval keeps every move for explicit approval
Limited autonomy executes only matching explicit rules
Smart and Limited modes do not create new category folders in the background
Trash always needs its own system confirmation
Moves and renames remain journaled and reversible when the original and current locations are safe
Pause settings changes and revocation update action permissions

## Structure strategy and present limits

The scanner first inventories safe directories and file metadata
It analyses all ordinary nested files and plans review actions where appropriate
Known files already matching a meaningful ancestor category folder can remain without a proposal
Unknown or potentially misfiled nested documents are shown in Review inbox
Nested suggestions always require explicit review so recursive analysis does not automatically flatten collections
Software projects app bundles common secret paths hidden entries and dataless cloud placeholders are excluded from traversal
Explicitly selected roots still pass the existing protected component checks

The planner can reuse exact category directory names or declared aliases
Examples include College for Education Work for Professional and Photos for Media
An exact category name takes priority over aliases
Multiple matching destinations are kept for review
When no matching directory exists the plan can request a new canonical category folder
This is a deterministic folder name strategy rather than semantic comprehension of arbitrary folder trees
It does not merge collections rename existing directories move whole software projects or redesign every Finder location

Current native scans have a limit of ten thousand entries and thirty two directory levels
An incomplete scan prevents automatic setup or file actions
File moves remain limited to stable single link regular files at most sixty four MiB
Local semantic content suggestions are inspected separately and always require review
Duplicate analysis is still an explicit bounded check under Your folders
No duplicate is automatically discarded

## Implementation map

The native workflow module owns persistent preferences reviewed setup tokens needed directory planning and live inventory snapshots
The Rust scanner owns safe traversal and structure aware destination selection
The Swift bridge owns multi selection bookmarks and system login item registration
The React app separates Setup Preferences and Review components from the overview and navigation

Preferences and the action policy revision are saved in one database transaction
Legacy commands that enabled a root without a reviewed preview token are no longer exposed to the webview
Setup and background directory preparation check the current cancellation epoch before changes

## Verification boundaries

Synthetic tests cover read only preview first setup stale tokens changed files changed folders policy and grant changes existing collections ambiguous destinations projects symlinks cancelled setup live inbox state and preference transaction rollback
Browser tests use a synthetic native bridge and cover setup automatic inbox population move approval Undo mode changes learning local inspection Trash cancellation filenames pause and small screens
The screenshots saved by the workflow use synthetic filenames and paths
Real macOS folder dialogs startup permissions sleep wake and cloud provider behavior still require the Mac acceptance checklist
The app remains a development alpha pending signing notarization and independent security review

## Platform reference

Apple documents multi selection through NSOpenPanel and recommends reading the urls collection when several locations are selected

https://developer.apple.com/documentation/appkit/nsopenpanel/allowsmultipleselection
