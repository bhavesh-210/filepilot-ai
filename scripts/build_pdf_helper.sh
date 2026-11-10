#!/bin/bash
set -euo pipefail
helper_temp=$(mktemp -d)
trap 'rm -rf "$helper_temp"' EXIT
mkdir -p desktop/src-tauri/parser
for helper_arch in arm64 x86_64
do
  xcrun swiftc -O -target "${helper_arch}-apple-macos13.0" desktop/src-tauri/macos/PdfWorker.swift -o "$helper_temp/$helper_arch" -framework PDFKit -framework Foundation
 done
lipo -create "$helper_temp/arm64" "$helper_temp/x86_64" -output desktop/src-tauri/parser/filepilot-pdf
chmod 755 desktop/src-tauri/parser/filepilot-pdf
