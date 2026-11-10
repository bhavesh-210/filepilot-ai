"""Run repository checks with optional local ML diagnostics and emit JSON"""
import argparse
import hashlib
import importlib.metadata
import io
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import tomllib
import unittest
from . import __version__
from .dataset import load_records, validate_records
from .report import validate_report
from .review import validate_review_pool

DATASETS = ("starter-v0.1.0", "expanded-v0.1.0")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run_test_suite(root, with_ml, with_pdf=False):
    discovered = unittest.TestLoader().discover(str(root / "tests"))
    def flatten(suite):
        for item in suite:
            if isinstance(item, unittest.TestSuite):
                yield from flatten(item)
            else:
                yield item
    all_tests = list(flatten(discovered))
    tests = [t for t in all_tests
             if (with_ml or not t.id().startswith("test_research_baseline."))
             and (with_pdf or not t.id().startswith("test_pdf.PdfExtractionTests."))]
    require(bool(tests), "No tests were discovered")
    result = unittest.TextTestRunner(stream=io.StringIO(), verbosity=2).run(unittest.TestSuite(tests))
    return dict(passed=result.wasSuccessful(), tests_run=result.testsRun, skipped=len(result.skipped),
                omitted_optional_tests=len(all_tests) - len(tests),
                expected_failures=len(result.expectedFailures),
                failures=[dict(test=t.id(), traceback=trace) for t, trace in result.failures + result.errors],
                unexpected_successes=[t.id() for t in result.unexpectedSuccesses])


def check_metadata(root):
    metadata = tomllib.loads((root / "pyproject.toml").read_text())
    require(metadata["project"]["version"] == __version__, "Package version mismatch")
    require(metadata["project"]["dependencies"] == [], "Scanner gained runtime dependencies")
    return dict(package_version=__version__, runtime_dependencies=[])


def check_expansion(original, expanded):
    by_id = {r["id"]: r for r in expanded}
    require(all(by_id.get(r["id"]) == r for r in original), "Expansion changed an original record")
    identifiers = {r["id"] for r in original}
    added = [r for r in expanded if r["id"] not in identifiers]
    require(all(r["split"] == "train" for r in added), "Expansion added validation or test examples")
    return dict(original_records=len(original), added_training_records=len(added), original_splits_preserved=True)


def check_review_overlap(pool, datasets):
    known = {r["filename"].casefold() for rows in datasets.values() for r in rows}
    require(not known & {r["filename"].casefold() for r in pool}, "Review filenames overlap with existing datasets")
    return dict(exact_filename_overlap=False, semantic_independence_established=False)


def check_cli(root):
    with tempfile.TemporaryDirectory(prefix="filepilot-check-") as directory:
        fixture = Path(directory).resolve(strict=True)
        source = fixture / "lecture.txt"
        source.write_text("synthetic fixture", encoding="utf-8")
        before = source.read_bytes(), source.stat().st_mtime_ns
        environment = dict(os.environ, PYTHONPATH=str(root / "src"))
        process = subprocess.run([sys.executable, "-m", "filepilot.cli", str(fixture)],
                                 cwd=root, env=environment, text=True, capture_output=True, timeout=30)
        require(process.returncode == 0, "Synthetic CLI scan failed: " + process.stderr)
        report = json.loads(process.stdout)
        validate_report(report)
        require(report["summary"]["entry_count"] == 1, "Unexpected synthetic entry count")
        require(report["entries"][0]["classification"]["category"] == "Education", "Synthetic category mismatch")
        require((source.read_bytes(), source.stat().st_mtime_ns) == before, "Synthetic source changed")
        require(sorted(p.name for p in fixture.iterdir()) == ["lecture.txt"], "Unexpected fixture changes")
    return dict(explicit_synthetic_root=True, report_valid=True, fixture_unchanged=True)


def check_recorded_research(root, datasets):
    recorded = json.loads((root / "docs/research-results.json").read_text())
    require(recorded["test_split_scored"] is False and recorded["model_saved"] is False,
            "Recorded research scope changed")
    for name, rows in datasets.items():
        canonical = json.dumps(rows, sort_keys=True, ensure_ascii=True, separators=(",", ":")).encode()
        key = f"ai/datasets/{name}/records.jsonl"
        experiment = recorded["datasets"][key]
        require(experiment["records_sha256"] == hashlib.sha256(canonical).hexdigest(), "Recorded dataset hash is stale")
        for split in ("train", "validation"):
            count = sum(r["split"] == split and r["expected_policy"] != "leave_untouched" for r in rows)
            result = experiment["results"][split]
            require(result["examples"] == count, "Recorded example count mismatch")
            for method in ("rules", "ml"):
                metrics = result[method]
                require(sum(map(sum, metrics["confusion_matrix"])) == count, "Recorded confusion matrix mismatch")
                require(0 <= metrics["accuracy"] <= 1 and 0 <= metrics["macro_f1"] <= 1, "Recorded metric outside range")
    return dict(recorded_datasets=len(datasets), dataset_hashes_match=True,
                fresh_model_evaluation=False, test_split_scored=False)


def check_recorded_uncertainty(root):
    recorded = json.loads((root / "docs/uncertainty-results.json").read_text())
    require(recorded["test_split_scored"] is False and recorded["scores_calibrated"] is False,
            "Recorded uncertainty scope changed")
    require(recorded["production_threshold"] is None and bool(recorded["thresholds"]), "Unexpected threshold selection")
    for row in recorded["thresholds"]:
        count, accepted = row["examples"], row["suggestions"]
        require(count > 0 and accepted + row["review_count"] == count, "Threshold count mismatch")
        require(0 <= row["wrong_suggestions"] <= accepted, "Wrong suggestion count mismatch")
        require(math.isclose(row["coverage"], accepted / count), "Threshold coverage mismatch")
        expected = (accepted - row["wrong_suggestions"]) / accepted if accepted else None
        require(row["suggestion_accuracy"] == expected, "Threshold accuracy mismatch")
    return dict(thresholds=len(recorded["thresholds"]), production_threshold=None, scores_calibrated=False)


def evaluate_ml(root, datasets):
    from .research_baseline import run
    from .abstention import evaluate
    result = {}
    for name, rows in datasets.items():
        scores, _ = run(rows, research_only=True)
        result[name] = {split: {"examples": values["examples"],
            "rules_accuracy": values["rules"]["accuracy"], "ml_accuracy": values["ml"]["accuracy"],
            "ml_macro_f1": values["ml"]["macro_f1"], "ml_errors": values["ml"]["errors"]}
            for split, values in scores.items()}
    return dict(dependencies={name: importlib.metadata.version(name) for name in ("scikit-learn", "numpy", "scipy")},
                datasets=result, uncertainty=evaluate(datasets["expanded-v0.1.0"], research_only=True),
                test_split_scored=False, model_saved=False)


def collect_checks(root, with_ml=False, include_tests=True, with_pdf=False):
    root = Path(root).resolve(strict=True)
    checks = []
    def record(name, operation):
        try:
            details = operation()
            passed = not isinstance(details, dict) or details.get("passed", True)
            checks.append(dict(name=name, status="passed" if passed else "failed", details=details))
            return details if passed else None
        except Exception as exc:
            checks.append(dict(name=name, status="failed", error=type(exc).__name__ + ": " + str(exc)))
            return None
    record("package metadata", lambda: check_metadata(root))
    if include_tests:
        record("automated tests", lambda: run_test_suite(root, with_ml, with_pdf))
    datasets = {}
    for name in DATASETS:
        def load_and_validate(name=name):
            rows = load_records(root / "ai/datasets" / name / "records.jsonl")
            summary = validate_records(rows)
            datasets[name] = rows
            return summary
        record("dataset " + name, load_and_validate)
    if len(datasets) == len(DATASETS):
        record("expansion preserves source data", lambda: check_expansion(datasets[DATASETS[0]], datasets[DATASETS[1]]))
        record("recorded research report", lambda: check_recorded_research(root, datasets))
    def review_summary():
        pool = load_records(root / "ai/datasets/review-pool-v0.1.0/records.jsonl")
        summary = validate_review_pool(pool)
        if len(datasets) == len(DATASETS):
            summary["overlap_check"] = check_review_overlap(pool, datasets)
        return summary
    record("development review pool", review_summary)
    record("recorded uncertainty report", lambda: check_recorded_uncertainty(root))
    record("synthetic CLI scan", lambda: check_cli(root))
    if with_pdf:
        record("optional PDF dependency", lambda: dict(pypdf=importlib.metadata.version("pypdf")))
    if with_ml:
        record("fresh local ML diagnostics", lambda: evaluate_ml(root, datasets))
    else:
        checks.append(dict(name="fresh local ML diagnostics", status="not_requested",
                           reason="Use --with-ml with optional research dependencies installed"))
    return dict(schema_version="0.1.0", mode="research" if with_ml else ("documents" if with_pdf else "core"),
                pdf_checks_requested=with_pdf,
                python=platform.python_version(), passed=all(c["status"] != "failed" for c in checks),
                test_split_scored=False, checks=checks,
                limitations=["Synthetic labels remain provisional", "Human review is pending",
                             "No production confidence threshold or filesystem action permissions"])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--with-ml", action="store_true", help="Also run optional synthetic research diagnostics")
    parser.add_argument("--with-pdf", action="store_true", help="Also run optional PDF extraction tests")
    args = parser.parse_args(argv)
    report = collect_checks(Path(__file__).resolve().parents[2], args.with_ml, with_pdf=args.with_pdf)
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
