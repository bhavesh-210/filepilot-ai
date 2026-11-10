use filepilot_core::{Engine, Settings};
use rusqlite::Connection;
use std::{
    fs::{self, File, FileTimes},
    time::{Duration, SystemTime},
};
fn setup() -> (tempfile::TempDir, Engine, i64) {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::new(Connection::open_in_memory().unwrap()).unwrap();
    let id = e
        .authorize(d.path().canonicalize().unwrap(), "test".into())
        .unwrap();
    for name in ["lecture.txt", "receipt.txt"] {
        fs::write(d.path().join(name), "synthetic").unwrap();
        File::options()
            .write(true)
            .open(d.path().join(name))
            .unwrap()
            .set_times(FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(60)))
            .unwrap();
    }
    e.mark_initial_reviewed(id).unwrap();
    (d, e, id)
}
#[test]
fn collection_preview_is_read_only_and_moves_are_undoable() {
    let (d, mut e, id) = setup();
    let p = e
        .plan_collection(
            id,
            vec!["lecture.txt".into(), "receipt.txt".into()],
            "Important documents".into(),
            true,
        )
        .unwrap();
    assert!(!d.path().join("Important documents").exists());
    assert!(e.history().unwrap().is_empty());
    let result = e.apply_collection(id, p.token, || true).unwrap();
    assert_eq!(result.operations.len(), 2);
    assert!(result.errors.is_empty());
    assert!(d.path().join("Important documents/lecture.txt").exists());
    for op in result.operations {
        e.undo(op).unwrap();
    }
    assert!(d.path().join("lecture.txt").exists());
    assert!(d.path().join("receipt.txt").exists());
}
#[test]
fn selected_files_can_be_added_to_existing_nested_collection() {
    let (d, mut e, id) = setup();
    fs::create_dir_all(d.path().join("College/Important")).unwrap();
    let p = e
        .plan_collection(
            id,
            vec!["lecture.txt".into()],
            "College/Important".into(),
            false,
        )
        .unwrap();
    assert_eq!(
        e.apply_collection(id, p.token, || true)
            .unwrap()
            .operations
            .len(),
        1
    );
    assert!(d.path().join("College/Important/lecture.txt").exists());
}
#[test]
fn collisions_duplicates_and_invalid_titles_are_rejected() {
    let (d, mut e, id) = setup();
    for title in ["../Escape", ".ssh", "Project/Inner", " Name ", "Library"] {
        assert!(e
            .plan_collection(id, vec!["lecture.txt".into()], title.into(), true)
            .is_err());
    }
    fs::create_dir(d.path().join("Important")).unwrap();
    fs::write(d.path().join("Important/lecture.txt"), "keep").unwrap();
    assert!(e
        .plan_collection(id, vec!["lecture.txt".into()], "Important".into(), false)
        .is_err());
    assert!(e
        .plan_collection(id, vec!["lecture.txt".into(); 2], "New".into(), true)
        .is_err());
    assert_eq!(
        fs::read_to_string(d.path().join("Important/lecture.txt")).unwrap(),
        "keep"
    );
}
#[test]
fn same_named_files_from_different_folders_never_overwrite() {
    let (d, mut e, id) = setup();
    fs::create_dir(d.path().join("Other")).unwrap();
    fs::copy(
        d.path().join("lecture.txt"),
        d.path().join("Other/lecture.txt"),
    )
    .unwrap();
    let times = FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(60));
    File::options()
        .write(true)
        .open(d.path().join("Other/lecture.txt"))
        .unwrap()
        .set_times(times)
        .unwrap();
    assert!(e
        .plan_collection(
            id,
            vec!["lecture.txt".into(), "Other/lecture.txt".into()],
            "New".into(),
            true
        )
        .is_err());
}
#[test]
fn changed_source_or_occupied_folder_invalidates_confirmation() {
    for changed_source in [true, false] {
        let (d, mut e, id) = setup();
        let p = e
            .plan_collection(id, vec!["lecture.txt".into()], "Important".into(), true)
            .unwrap();
        if changed_source {
            fs::write(d.path().join("lecture.txt"), "changed").unwrap()
        } else {
            fs::create_dir(d.path().join("Important")).unwrap()
        }
        assert!(e.apply_collection(id, p.token, || true).is_err());
        assert!(d.path().join("lecture.txt").exists());
    }
}
#[test]
fn permission_and_policy_changes_cancel_collection() {
    let (d, mut e, id) = setup();
    let p = e
        .plan_collection(id, vec!["lecture.txt".into()], "Important".into(), true)
        .unwrap();
    assert!(e.apply_collection(id, p.token, || false).is_err());
    assert!(!d.path().join("Important").exists());
    e.set_settings(Settings {
        monitoring: false,
        ..e.settings.clone()
    })
    .unwrap();
    assert!(e.apply_collection(id, p.token, || true).is_err());
}
#[test]
fn project_symlink_and_outside_source_are_protected() {
    let (d, mut e, id) = setup();
    fs::create_dir(d.path().join("project")).unwrap();
    fs::write(d.path().join("project/package.json"), "{}").unwrap();
    fs::copy(
        d.path().join("lecture.txt"),
        d.path().join("project/lecture.txt"),
    )
    .unwrap();
    std::os::unix::fs::symlink("lecture.txt", d.path().join("link.txt")).unwrap();
    for source in ["project/lecture.txt", "link.txt", "../outside"] {
        assert!(e
            .plan_collection(id, vec![source.into()], "Important".into(), true)
            .is_err());
    }
    assert!(e
        .plan_collection(id, vec!["lecture.txt".into()], "project".into(), false)
        .is_err());
}
#[test]
fn collection_never_grants_automatic_move_permission() {
    let (_d, mut e, id) = setup();
    e.create_categories(id).unwrap();
    let p = e
        .plan_collection(id, vec!["lecture.txt".into()], "Education".into(), false)
        .unwrap();
    let inventory = e.scan_background(id).unwrap();
    let row = inventory
        .proposals
        .iter()
        .find(|row| row.source == "lecture.txt")
        .unwrap();
    assert_eq!(row.id, p.proposals[0].id);
    assert!(!row.certain);
    assert!(e.apply(row.id, false).is_err());
    e.cancel_collection(id, p.token).unwrap();
    assert!(e.apply(row.id, true).is_err());
}
#[test]
fn title_suggestions_use_selected_names_and_are_editable() {
    let (_d, e, id) = setup();
    assert_eq!(
        e.collection_title(id, &["lecture.txt".into()]).unwrap(),
        "Important study files"
    );
    assert_eq!(
        e.collection_title(id, &["lecture.txt".into(), "receipt.txt".into()])
            .unwrap(),
        "Important files"
    );
    assert!(e.collection_title(id, &[]).is_err());
}
#[test]
fn full_preflight_prevents_partial_moves_on_changed_second_file() {
    let (d, mut e, id) = setup();
    let p = e
        .plan_collection(
            id,
            vec!["lecture.txt".into(), "receipt.txt".into()],
            "Important".into(),
            true,
        )
        .unwrap();
    fs::write(d.path().join("receipt.txt"), "changed").unwrap();
    assert!(e.apply_collection(id, p.token, || true).is_err());
    assert!(d.path().join("lecture.txt").exists());
    assert!(!d.path().join("Important").exists());
}
#[test]
fn cancellation_after_first_move_reports_partial_work_and_keeps_undo() {
    use std::cell::Cell;
    let (d, mut e, id) = setup();
    let p = e
        .plan_collection(
            id,
            vec!["lecture.txt".into(), "receipt.txt".into()],
            "Important".into(),
            true,
        )
        .unwrap();
    let calls = Cell::new(0);
    let result = e
        .apply_collection(id, p.token, || {
            let next = calls.get() + 1;
            calls.set(next);
            next <= 3
        })
        .unwrap();
    assert_eq!(result.operations.len(), 1);
    assert_eq!(result.errors.len(), 1);
    assert!(d.path().join("receipt.txt").exists());
    e.undo(result.operations[0]).unwrap();
    assert!(d.path().join("lecture.txt").exists());
}
