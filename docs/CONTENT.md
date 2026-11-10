# Inspecting document contents

Content inspection is separate from the metadata scanner and only reads one explicitly selected file

There is no default folder and the existing scanner does not inspect document contents

The first supported formats are text Markdown CSV and log files

UTF 8 and UTF 16 with a byte order mark are supported and unknown encodings are rejected

## Privacy and control

The JSON report hides extracted text by default

Including the text requires an explicit option

Document contents are data rather than instructions and no discovered file is executed

Nothing is moved renamed or deleted

## Safety boundaries

The inspector rejects traversal paths symlinks special files hardlinks known sensitive paths and detected project boundaries

It checks file identity and metadata before and after a bounded read

A changed file produces no extracted text report

These checks are not an atomic snapshot or a complete sensitive file detector

The default input limit is 256 KiB and the hard limit is 2 MiB

Returned text is limited to 16000 characters and truncation is reported

The SF DATALESS flag is checked on files and intermediate directories

File providers can react to metadata access so this prototype cannot promise that inspecting a provider managed folder will never trigger downloads

Use local synthetic files while testing

## Explicit inspection

```sh
PYTHONPATH=src python3 -m filepilot.content /absolute/path/to/test-folder note.txt
PYTHONPATH=src python3 -m filepilot.content /absolute/path/to/test-folder note.txt --include-text
```

The first command reports extraction metadata and the second also returns the text

The report always keeps review required and grants no permission to change files

Content extraction is not semantic understanding and is not connected to the trained classifier

## Technical references

https://developer.apple.com/documentation/technotes/tn3150-getting-ready-for-data-less-files

https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/stat.h
