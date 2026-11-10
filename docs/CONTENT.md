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

## Optional PDF inspection

PDF inspection requires a separate explicit opt in and the optional pypdf dependency
The pinned parser version is 7 0 1 under the BSD three clause license
The metadata scanner still never reads document contents

The parent supplies a bounded byte snapshot to a separate Python process
The worker receives no document path and uses no network APIs
A five second process timeout and four second CPU limit apply
Linux also applies a 512 MiB address space limit
macOS does not enforce an address space limit here
This process boundary is not an operating system sandbox
Malformed or adversarial PDFs can still stress the parser within those limits

The default page limit is five with a maximum of twenty
Decoded page streams larger than one MiB are rejected after decompression
Decompression itself may allocate memory before that check
Text output remains capped at sixteen thousand characters
Encrypted PDFs are skipped and image only documents require OCR which is not implemented
Embedded actions attachments and document instructions are not executed

The timeout covers parsing rather than filesystem reads
Cloud providers may respond to metadata requests despite placeholder checks
Tests use synthetic documents only

```sh
python -m filepilot.content examples/synthetic selected.pdf --pdf
python -m filepilot.check --with-pdf
```

Parser documentation
https://pypdf.readthedocs.io/en/latest/user/extract-text.html
https://pypdf.readthedocs.io/en/latest/meta/faq.html
