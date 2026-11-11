use filepilot_core::{CleanupProfile, Engine, Mode};
use rusqlite::Connection;
use std::{
    fs::{self, File, FileTimes},
    time::{Duration, SystemTime},
};
fn setup() -> (tempfile::TempDir, Engine, i64) {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::new(Connection::open_in_memory().unwrap()).unwrap();
    let id = e
        .authorize(d.path().canonicalize().unwrap(), "synthetic".into())
        .unwrap();
    e.mark_initial_reviewed(id).unwrap();
    write(d.path(), "lecture.txt");
    write(d.path(), "receipt.txt");
    (d, e, id)
}
fn write(root: &std::path::Path, name: &str) {
    let p = root.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, "Synthetic text").unwrap();
    File::options()
        .write(true)
        .open(p)
        .unwrap()
        .set_times(FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(120)))
        .unwrap();
}
fn profile(id: i64) -> CleanupProfile {
    CleanupProfile {
        root_ids: vec![id],
        by_year: false,
        date_prefix: false,
    }
}
#[test]
fn preview_and_selection_are_read_only() {
    let (d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert_eq!(p.rows.len(), 2);
    assert!(!d.path().join("Documents").exists());
    assert!(e.history().unwrap().is_empty());
    assert!(e.cleanup_is_held(id));
    let r = e
        .apply_cleanup(p.token, vec![(0, p.rows[0].destination.clone())], || true)
        .unwrap();
    assert_eq!(r.applied, 1);
    assert!(r.error.is_none());
    assert_eq!(e.cleanup_runs().unwrap()[0].operations, 1);
}
#[test]
fn nested_year_dates_and_batch_undo() {
    let (d, mut e, id) = setup();
    let mut spec = profile(id);
    spec.by_year = true;
    spec.date_prefix = true;
    let p = e.prepare_cleanup(spec).unwrap();
    assert!(p.rows[0].destination.split('/').count() == 3);
    let rows = p.rows.clone();
    let r = e
        .apply_cleanup(
            p.token,
            rows.iter()
                .enumerate()
                .map(|(i, r)| (i, r.destination.clone()))
                .collect(),
            || true,
        )
        .unwrap();
    assert_eq!(r.applied, 2);
    assert!(r.error.is_none());
    let u = e.undo_cleanup(r.run_id, || true).unwrap();
    assert_eq!(u.applied, 2);
    assert!(d.path().join("lecture.txt").exists());
    assert!(e.scan(id).unwrap().proposals.is_empty());
    assert_eq!(e.cleanup_runs().unwrap()[0].remaining, 0);
}
#[test]
fn changed_source_fails_before_creating_anything() {
    let (d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    write(d.path(), &p.rows[0].source);
    assert!(e
        .apply_cleanup(p.token, vec![(0, p.rows[0].destination.clone())], || true)
        .is_err());
    assert!(!d.path().join("Documents").exists());
    assert!(e.cleanup_runs().unwrap().is_empty());
}
#[test]
fn collision_and_duplicate_destinations_do_not_overwrite() {
    let (d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    write(d.path(), "Documents/lecture.txt");
    assert!(e
        .apply_cleanup(
            p.token,
            vec![
                (0, "Documents/lecture.txt".into()),
                (1, "Documents/lecture.txt".into())
            ],
            || true
        )
        .is_err());
    assert!(d.path().join("receipt.txt").exists());
}
#[test]
fn custom_destination_must_be_existing_or_previewed() {
    let (_d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert!(e
        .apply_cleanup(p.token, vec![(0, "Invented/secret.txt".into())], || true)
        .is_err());
}
#[test]
fn stale_policy_and_revocation_invalidate_preview() {
    for revoke in [true, false] {
        let (_d, mut e, id) = setup();
        let p = e.prepare_cleanup(profile(id)).unwrap();
        if revoke {
            e.revoke(id).unwrap();
        } else {
            e.set_settings(e.settings.clone()).unwrap();
        }
        assert!(e
            .apply_cleanup(p.token, vec![(0, p.rows[0].destination.clone())], || true)
            .is_err());
    }
}
#[test]
fn rename_setting_applies_to_edited_names() {
    let (_d, mut e, id) = setup();
    let mut s = e.settings.clone();
    s.rename = false;
    e.set_settings(s).unwrap();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert!(e
        .apply_cleanup(p.token, vec![(0, "Documents/new name.txt".into())], || true)
        .is_err());
}
#[test]
fn cancelled_run_does_not_move_files() {
    let (d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert!(e
        .apply_cleanup(p.token, vec![(0, p.rows[0].destination.clone())], || false)
        .is_err());
    assert!(d.path().join("lecture.txt").exists());
}
#[test]
fn undo_run_stops_on_changed_file_and_keeps_history() {
    let (d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    let r = e
        .apply_cleanup(
            p.token,
            p.rows
                .iter()
                .enumerate()
                .map(|(i, r)| (i, r.destination.clone()))
                .collect(),
            || true,
        )
        .unwrap();
    write(d.path(), &p.rows.last().unwrap().destination);
    let u = e.undo_cleanup(r.run_id, || true).unwrap();
    assert_eq!(u.applied, 0);
    assert!(u.error.is_some());
    assert_eq!(e.cleanup_runs().unwrap()[0].remaining, 2);
}
#[test]
fn saved_profile_maintains_future_files_and_smart_waits() {
    let (d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    e.save_cleanup_profile(p.token).unwrap();
    assert!(e.maintain_cleanup_profile(|| true).unwrap().is_none());
    e.cancel_cleanup(p.token);
    let r = e.maintain_cleanup_profile(|| true).unwrap().unwrap();
    assert_eq!(r.applied, 2);
    write(d.path(), "lecture next.txt");
    let mut s = e.settings.clone();
    s.mode = Mode::Smart;
    e.set_settings(s).unwrap();
    assert!(e.maintain_cleanup_profile(|| true).unwrap().is_none());
    assert!(d.path().join("lecture next.txt").exists());
    e.clear_cleanup_profile().unwrap();
    assert!(e.saved_cleanup_profile().unwrap().is_none());
}
#[test]
fn projects_and_existing_matching_collections_are_preserved() {
    let (d, mut e, id) = setup();
    write(d.path(), "demo/package.json");
    write(d.path(), "demo/src/receipt.txt");
    write(d.path(), "Finance/receipt retained.txt");
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert!(!p
        .rows
        .iter()
        .any(|r| r.source.contains("demo") || r.source.starts_with("Finance/")));
    assert!(p.protected > 0);
}
#[test]
fn destination_edits_remain_inside_root() {
    let (_d, mut e, id) = setup();
    for path in [
        "../outside.txt",
        "/tmp/outside.txt",
        "Documents/../../bad.txt",
    ] {
        let p = e.prepare_cleanup(profile(id)).unwrap();
        assert!(e
            .apply_cleanup(p.token, vec![(0, path.into())], || true)
            .is_err());
    }
}
#[test]
fn multiple_locations_are_one_plan_with_individual_root_boundaries() {
    let (d, mut e, a) = setup();
    let other = tempfile::tempdir().unwrap();
    write(other.path(), "lecture second.txt");
    let b = e
        .authorize(other.path().canonicalize().unwrap(), "other".into())
        .unwrap();
    e.mark_initial_reviewed(b).unwrap();
    let p = e
        .prepare_cleanup(CleanupProfile {
            root_ids: vec![a, b],
            by_year: false,
            date_prefix: false,
        })
        .unwrap();
    assert_eq!(p.rows.len(), 3);
    let r = e
        .apply_cleanup(
            p.token,
            p.rows
                .iter()
                .enumerate()
                .map(|(i, r)| (i, r.destination.clone()))
                .collect(),
            || true,
        )
        .unwrap();
    assert_eq!(r.applied, 3);
    assert!(d.path().join("Documents/lecture.txt").exists());
    assert!(other.path().join("Documents/lecture second.txt").exists());
}
#[test]
fn run_membership_survives_database_commit_failure_and_recovery() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("journal.sqlite");
    let mut e = Engine::new(Connection::open(&db).unwrap()).unwrap();
    let root = d.path().join("source");
    fs::create_dir(&root).unwrap();
    write(&root, "lecture.txt");
    let id = e
        .authorize(root.canonicalize().unwrap(), "test".into())
        .unwrap();
    e.mark_initial_reviewed(id).unwrap();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    let connection = Connection::open(&db).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_cleanup_commit BEFORE UPDATE ON operations WHEN NEW.state='applied' BEGIN SELECT RAISE(ABORT,'synthetic commit failure');END;").unwrap();
    let r = e
        .apply_cleanup(p.token, vec![(0, p.rows[0].destination.clone())], || true)
        .unwrap();
    assert_eq!(r.applied, 0);
    assert!(r.error.is_some());
    assert_eq!(e.cleanup_runs().unwrap()[0].operations, 1);
    assert!(root.join("Documents/lecture.txt").exists());
    connection
        .execute_batch("DROP TRIGGER fail_cleanup_commit")
        .unwrap();
    e.recover().unwrap();
    assert_eq!(e.cleanup_runs().unwrap()[0].remaining, 1);
    assert_eq!(e.undo_cleanup(r.run_id, || true).unwrap().applied, 1);
    assert!(root.join("lecture.txt").exists());
}
#[test]
fn unknown_types_remain_unselected() {
    let (d, mut e, id) = setup();
    write(d.path(), "puzzle.unrecognized");
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert!(
        !p.rows
            .iter()
            .find(|r| r.source == "puzzle.unrecognized")
            .unwrap()
            .selected
    );
}
#[test]
fn run_membership_failure_prevents_the_filesystem_move() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("journal.sqlite");
    let mut e = Engine::new(Connection::open(&db).unwrap()).unwrap();
    let root = d.path().join("source");
    fs::create_dir(&root).unwrap();
    write(&root, "lecture.txt");
    let id = e
        .authorize(root.canonicalize().unwrap(), "test".into())
        .unwrap();
    e.mark_initial_reviewed(id).unwrap();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    Connection::open(&db).unwrap().execute_batch("CREATE TRIGGER fail_membership BEFORE INSERT ON cleanup_operations BEGIN SELECT RAISE(ABORT,'synthetic membership failure');END;").unwrap();
    let r = e
        .apply_cleanup(p.token, vec![(0, p.rows[0].destination.clone())], || true)
        .unwrap();
    assert!(r.error.is_some());
    assert!(root.join("lecture.txt").exists());
    assert!(!root.join("Documents/lecture.txt").exists());
}
#[test]
fn failed_cleanup_does_not_enable_future_automation() {
    let (d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    write(d.path(), "Documents/lecture.txt");
    assert!(e
        .apply_cleanup_with_profile(
            p.token,
            vec![(0, p.rows[0].destination.clone())],
            true,
            || true
        )
        .is_err());
    assert!(e.saved_cleanup_profile().unwrap().is_none());
}
#[test]
fn successful_cleanup_can_save_the_approved_preset() {
    let (_d, mut e, id) = setup();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    let r = e
        .apply_cleanup_with_profile(
            p.token,
            vec![(0, p.rows[0].destination.clone())],
            true,
            || true,
        )
        .unwrap();
    assert_eq!(r.applied, 1);
    assert!(e.saved_cleanup_profile().unwrap().is_some());
}
#[test]
fn automatic_layout_skips_oversized_files_without_blocking_other_files() {
    let (d, mut e, id) = setup();
    let path = d.path().join("aaa large.txt");
    let f = File::create(&path).unwrap();
    f.set_len(65 * 1024 * 1024).unwrap();
    f.set_times(FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(120)))
        .unwrap();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert!(
        !p.rows
            .iter()
            .find(|r| r.source == "aaa large.txt")
            .unwrap()
            .selected
    );
    e.save_cleanup_profile(p.token).unwrap();
    e.cancel_cleanup(p.token);
    let result = e.maintain_cleanup_profile(|| true).unwrap().unwrap();
    assert_eq!(result.applied, 2);
    assert!(result.error.is_none());
    assert!(path.exists());
}
#[test]
fn automatic_layout_skips_newly_changing_files() {
    let (d, mut e, id) = setup();
    fs::write(
        d.path().join("aaa arriving.txt"),
        "Synthetic pending download",
    )
    .unwrap();
    let p = e.prepare_cleanup(profile(id)).unwrap();
    assert!(
        !p.rows
            .iter()
            .find(|r| r.source == "aaa arriving.txt")
            .unwrap()
            .selected
    );
    e.save_cleanup_profile(p.token).unwrap();
    e.cancel_cleanup(p.token);
    let result = e.maintain_cleanup_profile(|| true).unwrap().unwrap();
    assert_eq!(result.applied, 2);
    assert!(result.error.is_none());
    assert!(d.path().join("aaa arriving.txt").exists());
}
