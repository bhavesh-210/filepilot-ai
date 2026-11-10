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

Open Overview and choose your preferences in the one time setup
For the first test choose Smart approval
Choose one or several disposable test folders using the macOS picker
Select Understand my folders and expand the first cleanup preview
Read the folder structure source destination and reason for each proposal
Check which new category folders would be created
Select Start maintaining my folders only after reviewing that plan
If automatic startup was selected macOS may require login item approval
Open Review inbox and approve a single synthetic move
Verify its result in Finder
Open Activity and use Undo then verify that it restores the file
Full autonomy handles eligible future changes after setup without repeated scan clicks
The app maintains only the folders you explicitly allow

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

## Test collections

Open Collections after completing first cleanup setup
Choose the approved test folder and select disposable files
Choose an existing folder or leave the new folder title empty for a suggestion
Select Preview collection and review every source and new location
Use Edit selection to change the title or destination
Confirm the collection and verify the new location in Finder
Use Activity to undo individual moves

To test unwanted file cleanup select a disposable file in Collections
Select Move selected to Trash and cancel the native confirmation first
Verify the file remains in place
Then confirm a separate test Trash action and restore it with Activity Undo
