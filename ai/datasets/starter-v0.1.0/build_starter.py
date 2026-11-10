"""Rebuild only this authored fixture dataset, never inspect user folders.

Run explicitly: python3 ai/datasets/starter-v0.1.0/build_starter.py
Output is records.jsonl beside this script; existing output is replaced.
"""
import json
from pathlib import Path

# Each semantic family has two variants confined to the SAME split.
# Four train families, one validation family, one test family per category.
FAMILIES = {
    "Education": [("lecture_algebra", "pdf", "document"), ("assignment_graphs", "docx", "document"),
                  ("exam_calculus", "pdf", "document"), ("syllabus_physics", "txt", "document"),
                  ("homework_statistics", "pdf", "document"), ("college_chemistry", "pptx", "document")],
    "Development": [("web_api_routes", "ts", "source"), ("sorting_algorithm", "py", "source"),
                    ("database_adapter", "rs", "source"), ("mobile_screen", "swift", "source"),
                    ("queue_service", "go", "source"), ("tree_search", "cpp", "source")],
    "Professional": [("resume_engineer", "pdf", "document"), ("meeting_product", "docx", "document"),
                     ("proposal_design", "pdf", "document"), ("cv_developer", "docx", "document"),
                     ("contract_supplier", "pdf", "document"), ("meeting_recruitment", "txt", "document")],
    "Finance": [("invoice_equipment", "pdf", "document"), ("receipt_transport", "csv", "spreadsheet"),
                ("tax_summary", "xlsx", "spreadsheet"), ("salary_summary", "pdf", "document"),
                ("bank_summary", "pdf", "document"), ("invoice_services", "csv", "spreadsheet")],
    "Media": [("mountain_landscape", "jpg", "image"), ("city_timelapse", "mov", "video"),
              ("instrument_recording", "wav", "audio"), ("garden_panorama", "heic", "image"),
              ("concert_clip", "mp4", "video"), ("ambient_track", "flac", "audio")],
    "Software": [("editor_installer", "dmg", "software"), ("utility_installer", "pkg", "software"),
                 ("browser_installer", "dmg", "software"), ("driver_installer", "pkg", "software"),
                 ("terminal_installer", "dmg", "software"), ("player_installer", "pkg", "software")],
    "Personal": [("diary_travel", "txt", "document"), ("personal_goals", "docx", "document"),
                 ("journal_weekend", "md", "document"), ("diary_garden", "txt", "document"),
                 ("personal_recipes", "pdf", "document"), ("journal_reflections", "txt", "document")],
    "Unknown": [("scan_unidentified", "pdf", "document"), ("download_unidentified", "bin", "unknown"),
                ("archive_unidentified", "zip", "archive"), ("college_invoice", "pdf", "document"),
                ("document_untitled", "txt", "document"), ("resume_tax", "pdf", "document")],
}


def build():
    rows = []
    for category, families in FAMILIES.items():
        for index, (stem, ext, kind) in enumerate(families):
            split = "train" if index < 4 else "validation" if index == 4 else "test"
            for variant in ("draft", "final"):
                rows.append({"schema_version": "0.1.0", "id": f"{category.lower()}-{index+1}-{variant}",
                             "family_id": f"{category.lower()}-{stem}", "split": split,
                             "filename": f"{stem}_{variant}.{ext}", "file_type": kind,
                             "category": category,
                             "expected_policy": "review" if category == "Unknown" else "suggest_category",
                             "label_reason": "Ambiguous or conflicting metadata" if category == "Unknown" else
                                             f"Authored {category} scenario; provisional metadata label",
                             "provenance": {"kind": "synthetic", "source": "filepilot-authored-starter",
                                            "rights_status": "pending_review"}, "reviewed": False})
    # Project and safety examples: category and leave-untouched are independent labels.
    for split, suffix in (("train", "alpha"), ("validation", "beta"), ("test", "gamma")):
        rows.extend([
            {"schema_version": "0.1.0", "id": f"protected-project-{suffix}",
             "family_id": f"protected-project-{suffix}", "split": split,
             "filename": f"project_{suffix}_lecture.py", "file_type": "source", "category": "Development",
             "expected_policy": "leave_untouched", "label_reason": "Declared scenario: inside an intact coding project",
             "provenance": {"kind": "synthetic", "source": "filepilot-authored-starter", "rights_status": "pending_review"},
             "reviewed": False},
            {"schema_version": "0.1.0", "id": f"protected-secret-{suffix}",
             "family_id": f"protected-secret-{suffix}", "split": split,
             "filename": f"credential_{suffix}.key", "file_type": "unknown", "category": "Unknown",
             "expected_policy": "leave_untouched", "label_reason": "Declared scenario: sensitive key file; exclude",
             "provenance": {"kind": "synthetic", "source": "filepilot-authored-starter", "rights_status": "pending_review"},
             "reviewed": False}])
    return rows


if __name__ == "__main__":
    output = Path(__file__).with_name("records.jsonl")
    output.write_text("".join(json.dumps(row, ensure_ascii=False) + "\n" for row in build()), encoding="utf-8")
