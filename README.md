# FilePilot AI

Your files your Mac your control

I am building FilePilot AI to make it easier to keep files organised on a Mac

The idea is to understand what a file belongs to and help put it in the right place while keeping the user in control

## Where things stand

I have a working read only Python prototype that scans a selected test folder and suggests categories without changing any files

It reads basic metadata recognises common file types and spots coding projects so their structure can be preserved

I have also started testing a small local ML classifier using synthetic examples

I am adding this existing work to the repository in small steps

For now this repository contains the project overview and roadmap

The runnable code and tests will follow in separate commits

## What I want to build

- File analysis that works locally without cloud AI APIs
- A preview before the first cleanup
- Support for keeping coding projects together
- Different levels of control over automatic organisation
- A history of changes with an option to undo them
- Background monitoring for newly added files

Moving files to Trash or deleting them will always need separate confirmation

## How I am approaching it

I am starting with a read only scanner before building anything that moves or renames files

The current experiments use synthetic examples rather than personal documents

Privacy and reliability matter more to me than getting a quick demo working

The development roadmap is in the docs folder
