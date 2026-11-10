use filepilot_core::{Engine, Route};
use rusqlite::Connection;
use std::{
    fs::{self, File, FileTimes},
    path::Path,
    time::{Duration, SystemTime},
};
fn write(root: &Path, path: &str, text: &str) {
    let p = root.join(path);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, text).unwrap();
    File::options()
        .write(true)
        .open(p)
        .unwrap()
        .set_times(FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(60)))
        .unwrap();
}
fn setup() -> (tempfile::TempDir, Engine, i64, i64) {
    let d = tempfile::tempdir().unwrap();
    let base = d.path().canonicalize().unwrap();
    fs::create_dir(base.join("Downloads")).unwrap();
    fs::create_dir(base.join("Documents")).unwrap();
    let mut e = Engine::new(Connection::open(base.join("journal.sqlite")).unwrap()).unwrap();
    let a = e.authorize(base.join("Downloads"), "a".into()).unwrap();
    let b = e.authorize(base.join("Documents"), "b".into()).unwrap();
    e.mark_initial_reviewed(a).unwrap();
    e.mark_initial_reviewed(b).unwrap();
    write(
        &base.join("Downloads"),
        "Bundle/Term 1/lecture.txt",
        "university course assignment homework",
    );
    write(
        &base.join("Downloads"),
        "Bundle/Term 2/receipt.txt",
        "invoice payment bank amount",
    );
    (d, e, a, b)
}
#[test]
fn folder_move_preserves_tree_and_undo() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .unwrap();
    assert_eq!(p.files_count, 2);
    assert!(d
        .path()
        .join("Downloads/Bundle/Term 1/lecture.txt")
        .exists());
    assert!(e.history().unwrap().is_empty());
    let op = e.apply_folder_transfer(p.token, || true).unwrap();
    assert!(d
        .path()
        .join("Documents/Bundle/Term 1/lecture.txt")
        .exists());
    assert!(d
        .path()
        .join("Documents/Bundle/Term 2/receipt.txt")
        .exists());
    assert!(!d.path().join("Downloads/Bundle").exists());
    assert!(e.scan(b).unwrap().proposals.is_empty());
    e.undo(op).unwrap();
    assert!(d
        .path()
        .join("Downloads/Bundle/Term 2/receipt.txt")
        .exists());
    assert!(e.scan(a).unwrap().proposals.is_empty());
}
#[test]
fn projects_and_protected_entries_reject_entire_folder() {
    for entry in ["Code/package.json", ".env", "secret.key"] {
        let (d, mut e, a, b) = setup();
        write(
            &d.path().join("Downloads"),
            &format!("Bundle/{entry}"),
            "synthetic",
        );
        assert!(e
            .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
            .is_err());
        assert!(d
            .path()
            .join("Downloads/Bundle/Term 1/lecture.txt")
            .exists());
        assert!(e.history().unwrap().is_empty());
    }
}
#[test]
fn symlink_contents_are_never_transferred() {
    let (d, mut e, a, b) = setup();
    std::os::unix::fs::symlink(
        "Term 1/lecture.txt",
        d.path().join("Downloads/Bundle/link.txt"),
    )
    .unwrap();
    assert!(e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .is_err());
}
#[test]
fn preview_expires_if_any_nested_file_changes() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .unwrap();
    write(
        &d.path().join("Downloads"),
        "Bundle/Term 2/receipt.txt",
        "changed invoice",
    );
    assert!(e.apply_folder_transfer(p.token, || true).is_err());
    assert!(d
        .path()
        .join("Downloads/Bundle/Term 1/lecture.txt")
        .exists());
}
#[test]
fn collisions_and_directory_cycles_are_rejected() {
    let (d, mut e, a, b) = setup();
    fs::create_dir(d.path().join("Documents/Bundle")).unwrap();
    assert!(e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .is_err());
    assert!(e
        .prepare_folder_transfer(a, "Bundle".into(), a, "Bundle/Term 1".into())
        .is_err());
}
#[test]
fn both_grants_and_live_cancellation_are_required() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .unwrap();
    assert!(e.apply_folder_transfer(p.token, || false).is_err());
    assert!(d.path().join("Downloads/Bundle").exists());
    e.revoke(b).unwrap();
    assert!(e.apply_folder_transfer(p.token, || true).is_err());
}
#[test]
fn modified_destination_contents_block_undo() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .unwrap();
    let op = e.apply_folder_transfer(p.token, || true).unwrap();
    write(
        &d.path().join("Documents"),
        "Bundle/Term 1/lecture.txt",
        "changed content",
    );
    assert!(e.undo(op).is_err());
    assert!(d
        .path()
        .join("Documents/Bundle/Term 2/receipt.txt")
        .exists());
}
#[test]
fn recovery_handles_interrupted_move_and_undo() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .unwrap();
    let op = e.apply_folder_transfer(p.token, || true).unwrap();
    let db = Connection::open(d.path().join("journal.sqlite")).unwrap();
    db.execute("UPDATE operations SET state='pending' WHERE id=?", [op])
        .unwrap();
    e.recover().unwrap();
    assert_eq!(e.history().unwrap()[0].state, "applied");
    fs::rename(
        d.path().join("Documents/Bundle"),
        d.path().join("Downloads/Bundle"),
    )
    .unwrap();
    db.execute(
        "UPDATE operations SET state='undo_pending' WHERE id=?",
        [op],
    )
    .unwrap();
    e.recover().unwrap();
    assert_eq!(e.history().unwrap()[0].state, "undone");
    assert!(e.scan(a).unwrap().proposals.is_empty());
}
#[test]
fn folder_preview_holds_automatic_routes_until_cancelled() {
    let (_d, mut e, a, b) = setup();
    e.set_route(
        Route {
            category: "Finance".into(),
            target_root_id: b,
            folder: "Finance".into(),
            content: true,
        },
        true,
        || true,
    )
    .unwrap();
    e.scan(a).unwrap();
    let p = e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .unwrap();
    assert!(e.plan_routes().unwrap().is_empty());
    e.cancel_folder_transfer(p.token);
    assert!(!e.plan_routes().unwrap().is_empty());
}
#[test]
fn journal_commit_failure_remains_recoverable() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_folder_transfer(a, "Bundle".into(), b, "".into())
        .unwrap();
    let db = Connection::open(d.path().join("journal.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_commit BEFORE UPDATE OF state ON operations WHEN NEW.kind='folder_transfer' AND NEW.state='applied' BEGIN SELECT RAISE(FAIL,'synthetic failure'); END;").unwrap();
    assert!(e.apply_folder_transfer(p.token, || true).is_err());
    assert!(d
        .path()
        .join("Documents/Bundle/Term 1/lecture.txt")
        .exists());
    assert_eq!(e.history().unwrap()[0].state, "pending");
    db.execute_batch("DROP TRIGGER fail_commit").unwrap();
    e.recover().unwrap();
    assert_eq!(e.history().unwrap()[0].state, "applied");
}
