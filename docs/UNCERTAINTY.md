# Uncertain predictions

The abstention experiment is separate from the scanner and grants no permission to change files

It compares fixed score thresholds on validation examples and does not select a threshold automatically

The model scores are uncalibrated and must not be treated as probabilities of correctness

Unknown predictions low scores and ties are sent for review

Every research suggestion still requires review before any future file operation

## What is measured

Coverage is the number of eligible known category suggestions divided by all scored validation examples

Suggestion accuracy only measures eligible suggestions

When nothing is suggested accuracy is reported as null rather than perfect

The report also counts wrong suggestions known categories deferred and Unknown examples incorrectly suggested as known categories

## Running the comparison

```sh
PYTHONPATH=src python3 -m filepilot.abstention ai/datasets/expanded-v0.1.0/records.jsonl --research-only --json
```

## Recorded findings

The expanded validation set has 16 examples in eight declared families

Without a score threshold the model makes 16 suggestions and two are wrong

At a threshold of zero point two it still makes both wrong suggestions

At a threshold of zero point three it makes four correct suggestions and sends the other 12 examples for review

Ten of the examples sent for review have known category labels

Four correct suggestions are too little evidence to claim dependable behaviour

No production threshold has been selected

The test split is not scored and neither model weights nor user file contents are saved
