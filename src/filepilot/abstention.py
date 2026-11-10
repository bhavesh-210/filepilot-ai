"""Diagnostic score thresholds; not calibrated confidence or action permission."""
import argparse
import math
import json
from .dataset import load_records
from .research_baseline import features, run
from .report import CATEGORIES


def decide(scores, classes, threshold):
    if not math.isfinite(threshold) or not 0 <= threshold <= 1:
        raise ValueError("Threshold must be between 0 and 1")
    if not len(scores) or len(scores) != len(classes) or len(set(classes)) != len(classes):
        raise ValueError("Invalid score/class lengths")
    if not set(classes) <= CATEGORIES:
        raise ValueError("Invalid category")
    if any(not math.isfinite(p) or not 0 <= p <= 1 for p in scores) or not math.isclose(sum(scores), 1, abs_tol=1e-6):
        raise ValueError("Expected normalized finite scores")
    index = max(range(len(scores)), key=lambda i: scores[i])
    raw, score = str(classes[index]), float(scores[index])
    tied = sum(math.isclose(p, score, rel_tol=0, abs_tol=1e-12) for p in scores) > 1
    accepted = raw != "Unknown" and score >= threshold and not tied
    return {"raw_category": raw, "category": raw if accepted else "Unknown",
            "score": score, "suggestion_eligible": accepted, "review_required": True}


def summarize(rows, decisions):
    if not rows or len(rows) != len(decisions):
        raise ValueError("Nonempty matching rows and decisions required")
    accepted = [(r, d) for r, d in zip(rows, decisions) if d["suggestion_eligible"]]
    correct = sum(r["category"] == d["category"] for r, d in accepted)
    return {"examples": len(rows), "suggestions": len(accepted),
            "coverage": len(accepted) / len(rows),
            "suggestion_accuracy": correct / len(accepted) if accepted else None,
            "wrong_suggestions": len(accepted) - correct,
            "review_count": len(rows) - len(accepted),
            "known_category_deferred": sum(r["category"] != "Unknown" and not d["suggestion_eligible"] for r, d in zip(rows, decisions)),
            "unknown_category_suggested": sum(r["category"] == "Unknown" for r, d in accepted)}



def evaluate(rows, research_only=False, thresholds=(0, .2, .3, .4, .5)):
    for threshold in thresholds:
        if not math.isfinite(threshold) or not 0 <= threshold <= 1:
            raise ValueError("Threshold must be between 0 and 1")
    _, model = run(rows, research_only)
    selected = [r for r in rows if r["split"] == "validation" and r["expected_policy"] != "leave_untouched"]
    probabilities = model.predict_proba([features(r) for r in selected])
    summaries = []
    for threshold in thresholds:
        decisions = [decide(p, model.classes_, threshold) for p in probabilities]
        summaries.append(dict(threshold=float(threshold), **summarize(selected, decisions)))
    return dict(schema_version="0.1.0", split="validation", test_split_scored=False,
                scores_calibrated=False, production_threshold=None,
                limitation="Repeated synthetic development evaluation with eight declared validation families",
                thresholds=summaries)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dataset")
    parser.add_argument("--research-only", action="store_true")
    parser.add_argument("--thresholds", nargs="+", type=float, default=[0, .2, .3, .4, .5])
    parser.add_argument("--json", action="store_true", help="Emit a structured threshold report")
    args = parser.parse_args()
    try:
        result = evaluate(load_records(args.dataset), args.research_only, args.thresholds)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            print("VALIDATION ONLY using uncalibrated scores and no threshold chosen automatically")
            print("Threshold  Suggestions  Coverage  Accuracy  Wrong  Review  KnownDeferred  UnknownSuggested")
            for row in result["thresholds"]:
                accuracy = "N/A" if row["suggestion_accuracy"] is None else f"{row['suggestion_accuracy']:.1%}"
                print(f"{row['threshold']:9.2f}  {row['suggestions']:11d}  {row['coverage']:8.1%}  {accuracy:>8}  {row['wrong_suggestions']:5d}  {row['review_count']:6d}  {row['known_category_deferred']:13d}  {row['unknown_category_suggested']:16d}")
            print("Test split not scored and no model saved or scanner changes made")
    except (ValueError, OSError, ImportError) as exc:
        parser.exit(2, str(exc) + "\n")


if __name__ == "__main__":
    main()
