# Local classifier research

The first experiment compares word TF IDF with Logistic Regression against the existing rules

The scanner does not use this model

Only filenames and declared file types are input features

Labels reasons identifiers family names and split names are excluded from features

The vectorizer and classifier fit on training examples only

Validation is scored separately and the test split is not scored

Leave untouched scenarios are excluded from category metrics because they belong to the policy layer

## Running the experiment

Optional research dependencies are separate from the scanner

```sh
python3 -m pip install -r requirements-ml.txt
PYTHONPATH=src python3 -m filepilot.research_baseline ai/datasets/starter-v0.1.0/records.jsonl ai/datasets/expanded-v0.1.0/records.jsonl --research-only
```

Add the JSON option for a machine readable report

```sh
PYTHONPATH=src python3 -m filepilot.research_baseline ai/datasets/expanded-v0.1.0/records.jsonl --research-only --json
```

The research only option permits this internal diagnostic with provisional synthetic labels

It does not approve rights or labels and no model is exported

The report records dependency versions configuration dataset hashes family counts category metrics confusion counts and individual errors

## Recorded results

The results file records the experiment rather than a claim about real user documents

The starter set has 64 scored training examples and 16 scored validation examples

The expanded set has 96 scored training examples and the same 16 validation examples

Rules agree with all starter validation labels while the first ML run agrees with 62 point 5 percent

The expanded ML run agrees with 87 point 5 percent and still mislabels the two untitled document variants

Training expansion followed inspection of validation errors so these are development results

The declared families and small sample size do not establish independent generalisation

There is no calibrated confidence score or production threshold
