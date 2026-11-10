"""Offline provisional-data research experiment; no filesystem actions."""
import argparse
import re
import hashlib
import importlib.metadata
import json
import platform
from collections import Counter
from .dataset import load_records, validate_records
from .report import CATEGORIES
from .scanner import classify


def features(row):
    return re.sub(r"[\W_]+", " ", row["filename"].lower()) + " type " + row["file_type"]


def score_predictions(rows, predictions):
    if not rows or len(rows) != len(predictions):
        raise ValueError("Expected nonempty matching examples and predictions")
    from sklearn.metrics import accuracy_score, classification_report, confusion_matrix, f1_score
    actual = [r["category"] for r in rows]
    labels = sorted(CATEGORIES)
    families = Counter(r["family_id"] for r in rows)
    family_correct = Counter(r["family_id"] for r, p in zip(rows, predictions) if r["category"] == p)
    return dict(
        accuracy=float(accuracy_score(actual, predictions)),
        macro_f1=float(f1_score(actual, predictions, labels=labels, average="macro", zero_division=0)),
        family_count=len(families),
        family_mean_accuracy=sum(family_correct[f] / count for f, count in families.items()) / len(families),
        confusion_labels=labels,
        confusion_matrix=confusion_matrix(actual, predictions, labels=labels).tolist(),
        per_category=classification_report(actual, predictions, labels=labels, output_dict=True, zero_division=0),
        errors=[(r["filename"], r["category"], p) for r, p in zip(rows, predictions) if r["category"] != p],
    )


def run(rows, research_only=False):
    status = validate_records(rows)
    if not status["training_eligible"] and not research_only:
        raise ValueError("Provisional labels: supply --research-only for this experiment")
    from sklearn.feature_extraction.text import TfidfVectorizer
    from sklearn.linear_model import LogisticRegression
    from sklearn.pipeline import Pipeline
    train = [r for r in rows if r["split"] == "train" and r["expected_policy"] != "leave_untouched"]
    if {r["category"] for r in train} != CATEGORIES:
        raise ValueError("Training requires all eight categories")
    model = Pipeline([
        ("tfidf", TfidfVectorizer(ngram_range=(1, 2), token_pattern=r"(?u)\b\w+\b")),
        ("lr", LogisticRegression(C=1.0, max_iter=1000, class_weight="balanced", random_state=42)),
    ])
    model.fit([features(r) for r in train], [r["category"] for r in train])
    results = {}
    for split in ("train", "validation"):
        selected = [r for r in rows if r["split"] == split and r["expected_policy"] != "leave_untouched"]
        if not selected:
            raise ValueError("Missing category examples in " + split)
        predictions = {"rules": [classify(r["filename"], r["file_type"])["category"] for r in selected],
                       "ml": model.predict([features(r) for r in selected]).tolist()}
        results[split] = {"examples": len(selected)}
        for name, values in predictions.items():
            results[split][name] = score_predictions(selected, values)
    return results, model


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("datasets", nargs="+")
    parser.add_argument("--research-only", action="store_true")
    parser.add_argument("--json", action="store_true", help="Emit a structured experiment report")
    args = parser.parse_args()
    try:
        experiments = {}
        for path in args.datasets:
            rows = load_records(path)
            results, _ = run(rows, args.research_only)
            canonical = json.dumps(rows, sort_keys=True, ensure_ascii=True, separators=(",", ":")).encode()
            experiments[path] = dict(records_sha256=hashlib.sha256(canonical).hexdigest(), results=results)
        report = dict(schema_version="0.1.0", experiment="word_tfidf_logistic_regression_v1",
            python=platform.python_version(), dependencies={name: importlib.metadata.version(name)
                for name in ("scikit-learn", "numpy", "scipy")},
            configuration=dict(word_ngrams=[1, 2], C=1.0, max_iter=1000, class_weight="balanced", random_state=42),
            research_only=args.research_only, test_split_scored=False, model_saved=False,
            limitation="Provisional synthetic labels and repeatedly inspected validation data",
            datasets=experiments)
        if args.json:
            print(json.dumps(report, indent=2))
        else:
            for path, experiment in experiments.items():
                print("\nDATASET:", path)
                for split, values in experiment["results"].items():
                    print(split.upper(), values["examples"], "examples")
                    for name in ("rules", "ml"):
                        score = values[name]
                        print(f"  {name}: accuracy={score['accuracy']:.1%}, macro-F1={score['macro_f1']:.3f}")
                print("ML validation mistakes:", experiment["results"]["validation"]["ml"]["errors"])
            print("\nTest split not scored and no model saved or scanner changes made")
    except (OSError, ValueError, ImportError) as exc:
        parser.exit(2, str(exc) + "\n")


if __name__ == "__main__":
    main()
