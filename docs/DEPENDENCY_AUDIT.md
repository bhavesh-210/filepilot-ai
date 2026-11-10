# Dependency audit

The initial Rust scan found RUSTSEC 2026 0459 in the bundled SQLite dependency
The application and core now use rusqlite version 0 41 1 and libsqlite3 sys version 0 39 1
The regression suite verifies that the loaded SQLite version contains the required security fix

The fixed core lockfile and desktop lockfile report zero known vulnerabilities against the advisory database snapshot recorded in dependency audit json
The frontend npm audit reports zero known vulnerabilities

## Informational findings

The all platform desktop lockfile retains two advisories

| Package | Advisory | Kind | Mac graph check |
| --- | --- | --- | --- |
| proc macro error version 1 0 4 | RUSTSEC 2024 0370 | Unmaintained | Absent from both selected Mac targets |
| glib version 0 18 5 | RUSTSEC 2024 0429 | Unsound iterator implementation | Absent from both selected Mac targets |

The selected targets are Apple silicon and Intel macOS
These findings are retained rather than silently suppressed
Linux desktop distribution is not supported by this release and would require a separate review of these dependencies

The automated dependency workflow checks both Rust lockfiles and the frontend lockfile
Any known vulnerability makes the relevant check fail
Informational warnings remain in the saved report

This checks published dependency advisories rather than proving the whole application secure
It does not cover operating system frameworks real permission behaviour filesystem races or independent penetration testing
Signing notarization Mac acceptance and the other release gates still apply

https://rustsec.org/advisories/RUSTSEC-2026-0459.html
https://rustsec.org/advisories/RUSTSEC-2024-0370.html
https://rustsec.org/advisories/RUSTSEC-2024-0429.html
https://github.com/rusqlite/rusqlite/releases/tag/v0.41.1
