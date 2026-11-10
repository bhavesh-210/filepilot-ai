# Changelog

## Measure uncertain predictions

- Record coverage wrong suggestions and review counts
- Keep unknown and tied predictions for review
- Report no accuracy when every example is deferred
- Leave the production threshold unset

## Compare the first local classifier

- Add an optional TF IDF and Logistic Regression experiment
- Fit on training data only and leave the test split unscored
- Record category metrics family counts confusion matrices and mistakes
- Keep experimental predictions separate from the scanner

## Check the synthetic datasets

- Add the starter and expanded synthetic examples
- Validate labels provenance and family splits
- Reject malformed provenance and duplicate JSON keys
- Keep incomplete category coverage ineligible for training

## Update the GitHub action runtimes

- Move checkout and Python setup to versions that use Node 24
- Keep both actions pinned to their verified commits
- Remove the deprecated Node 20 action configuration

## Run scanner checks on GitHub

- Run tests after each push to main
- Check Python 3 point 12 and 3 point 14 on Linux and macOS
- Keep the workflow read only and pin the action versions

## Add the read only scanner

- Add an explicit directory scan with JSON output
- Preserve coding project boundaries
- Skip symlinks special files and selected sensitive or generated folders
- Keep all category suggestions advisory
- Validate report structure and summary counts
- Add 21 focused synthetic tests

## Start the repository

- Add the project overview
- Add the development roadmap
- Ignore local environments backups credentials and generated artifacts
