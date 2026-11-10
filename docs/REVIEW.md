# Reviewing new examples

The review pool has 40 AI authored synthetic filenames and no document contents

All labels start empty and no model predictions are shown

These examples were authored after observing earlier errors so they are development data rather than an independently authored final test

## Recording a review

For each row record a category the actual reviewer whether the review was human or AI an evidence judgment and a short rationale

Use Unknown when the filename does not support a purpose and do not invent hidden contents

AI labels must be marked as AI and silence must never become approval

The validator keeps incomplete reviews separate from examples ready for development scoring

Even a complete human review does not turn this authored pool into an independent final test

The model training validator rejects review pool records because their schema is deliberately different

## Checking the pool

```sh
PYTHONPATH=src python3 -m filepilot.review ai/datasets/review-pool-v0.1.0/records.jsonl
```

A structurally valid pool may still have pending labels

No real person has supplied labels for this pool yet and that work remains pending

This step validates the review workflow rather than claiming that independent ground truth exists
