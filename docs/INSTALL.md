# Test the development app without a terminal

This is a development alpha rather than a signed production release
Use a disposable local folder and keep personal folders out of the first test

## Get the app

Open the native app checks workflow in GitHub
Choose the latest successful run for main
Download the FilePilot macOS development artifact from the Artifacts section
Extract the downloaded archive and then the inner development archive if GitHub adds an outer archive
The inner archive preserves the app bundle executable permissions

Move FilePilot AI app to Applications before testing login item registration
Open it through the normal macOS application launch process
The current build has no Developer ID notarization
If macOS blocks it do not remove quarantine attributes disable Gatekeeper or weaken system security
For a trusted build from this repository review Apple guidance before deciding whether to authorize that individual app
https://support.apple.com/en-in/102445
Production signing and normal installation acceptance are still release gates

https://github.com/bhavesh-210/filepilot-ai/actions/workflows/native-checks.yml

## Prepare harmless test files

In Finder open your Home folder and make a new folder called FilePilot Test
Use a local folder outside iCloud Dropbox and other sync providers
If you have cloned the project copy the synthetic example folder into that new test folder
Copy it rather than testing against the original repository fixtures
You can also create a few disposable files with names such as lecture notes txt and receipt txt
Do not copy real financial documents credentials or private work into the first test

## Review before enabling organization

Choose the test folder through the app folder picker
Create the category folders when you are ready
Scan and read the source destination and reason for each proposal
For individual control choose Smart approval
Approve a single synthetic move and verify its result in Finder
Use Undo and verify that it restores the file
Full autonomy starts only after the initial preview is explicitly acknowledged

## Test the other controls

Try changing modes while proposals are pending
Try turning filename normalization off
Try an explicit category preference and a remembered file example
Nearest example suggestions still need manual move approval
Use Inspect locally for a supported synthetic text file or PDF
Use duplicate analysis with two copies of the same harmless file
Trash always opens a separate native confirmation
Cancel the first Trash request to verify that nothing is moved
Only then test a confirmed Trash action and FilePilot Undo

The Mac acceptance checklist records the remaining system tests
Do not assume that a compiled app or a green browser mock test proves permission sleep wake or cloud provider behaviour

See MAC_ACCEPTANCE and IMPLEMENTATION_STATUS before using personal folders
