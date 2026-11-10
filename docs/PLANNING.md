# Destination planning and policy

## Implemented

The planner consumes a validated metadata inventory and an explicitly supplied list of existing destination folders
It produces a separate read only plan without filesystem execution permissions
Candidates must be inventoried folders outside coding projects and excluded paths
The planner blocks proposed targets that collide after Unicode normalization and case folding
Incomplete inventories block all proposals
Already placed files and protected project contents stay untouched

## Destination baseline

Ranking combines filename word overlap with the declared category of the destination folder
The method is a lexical and category baseline rather than a semantic embedding model
Scores are not probabilities and every proposal requires review
Explicit local corrections can prefer an existing candidate but never grant permission
Unknown files without evidence receive no destination

The evaluation helper reports destination accuracy Recall at three and mean reciprocal rank
Synthetic test agreement is not a measure of real document accuracy
No independent destination benchmark has been collected

## Policy engine

The policy module is separate from ranking and has no filesystem APIs
Full autonomy permits only actions with current authorization safety evidence and sufficient certainty
Smart approval additionally requires explicit approval
Limited autonomy requires explicit approval or a previously authorized rule
Renaming can be disabled
Trash and permanent deletion require a separate confirmation in every mode
A proposal with an old or missing policy revision is rejected for review
Future execution must obtain current settings and reevaluate policy immediately before each action

The boolean evidence inputs are an internal contract for the future trusted backend
They must never be accepted directly from an imported report document or webview
A permitted policy decision is not a secure filesystem capability
The current planner always reports executable false

## Local correction history

The SQLite preference component stores explicit category to destination corrections within an app supplied scope
The trusted caller must bind the scope to the actual authorized root rather than reusing it across folders
No document contents are stored
Disabled personalization neither records nor applies corrections
Clearing history removes all logical correction records
This does not promise physical erasure from SSD storage database journals or backups
The caller owns the database connection and must place it in private application storage
The current review screen does not connect to this database

## Offline review screen

Open frontend index html in a browser to inspect the synthetic demo or an explicitly generated plan
The screen has no network calls external fonts analytics or filesystem execution bridge
Imported paths and reasons are rendered as text rather than markup
Review marks are held in memory and can be exported as a separate review choices report
Changing preview mode clears those marks
Exported review choices are not action approvals and cannot be executed by this prototype

This screen is a browser prototype rather than the final React and Tauri desktop application
It does not scan folders or request macOS permissions

## Native integration decision

Keep direct filesystem operations in a trusted Rust backend with its own root validation
Tauri webview capabilities do not replace Rust path validation or macOS permissions
Use system folder selection and security scoped bookmarks for native persistent access
No broad shell command filesystem or remote webview permissions should be exposed

Native moves require identity rechecks exclusive destination creation a durable operation journal and tested recovery
Cross volume moves require a separate copy verify commit design
Undo must verify that both the current source and former destination have not changed
No move executor or Undo implementation is claimed by this milestone

## Sources

https://tauri.app/security/capabilities/
https://tauri.app/plugin/file-system/
https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox
https://developer.apple.com/documentation/foundation/nsurl/startaccessingsecurityscopedresource()
