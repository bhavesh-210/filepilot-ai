use filepilot_core::{Engine, Mode, Preferences, Settings};
use rusqlite::Connection;
use std::{
    fs::{self, File, FileTimes},
    path::Path,
    time::{Duration, SystemTime},
};
fn stable(path: &Path) {
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(60)))
        .unwrap();
}
fn setup() -> (tempfile::TempDir, Engine, i64) {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::new(Connection::open_in_memory().unwrap()).unwrap();
    let id = e
        .authorize(d.path().canonicalize().unwrap(), "test".into())
        .unwrap();
    fs::write(d.path().join("lecture.txt"), "synthetic").unwrap();
    stable(&d.path().join("lecture.txt"));
    (d, e, id)
}
#[test]
fn first_plan_is_read_only_and_creates_only_needed_folder_after_confirmation() {
    let (d, mut e, id) = setup();
    let p = e.prepare_setup(id).unwrap();
    assert_eq!(p.create_folders, vec!["Education"]);
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1);
    assert!(e.apply(p.inventory.proposals[0].id, false).is_err());
    e.finish_setup(id, p.token).unwrap();
    assert!(d.path().join("Education").is_dir());
    assert!(!d.path().join("Finance").exists());
    assert!(d.path().join("lecture.txt").exists());
    let p = e.maintain(id).unwrap().proposals.remove(0);
    let op = e.apply(p.id, false).unwrap();
    e.undo(op).unwrap();
    assert!(d.path().join("lecture.txt").exists());
}
#[test]
fn changing_files_or_structure_expires_setup() {
    for folder_change in [true, false] {
        let (d, mut e, id) = setup();
        let p = e.prepare_setup(id).unwrap();
        if folder_change {
            fs::create_dir(d.path().join("College")).unwrap()
        } else {
            fs::write(d.path().join("lecture.txt"), "changed").unwrap()
        }
        assert!(e.finish_setup(id, p.token).is_err());
        assert!(!e.roots[&id].initial_reviewed);
        assert!(!d.path().join("Education").exists());
    }
}
#[test]
fn policy_and_authorization_changes_expire_setup() {
    let (d, mut e, id) = setup();
    let p = e.prepare_setup(id).unwrap();
    e.set_settings(Settings {
        mode: Mode::Smart,
        ..e.settings.clone()
    })
    .unwrap();
    assert!(e.finish_setup(id, p.token).is_err());
    let p = e.prepare_setup(id).unwrap();
    e.authorize(d.path().canonicalize().unwrap(), "renewed".into())
        .unwrap();
    assert!(e.finish_setup(id, p.token).is_err());
}
#[test]
fn existing_collections_are_inventoried_and_not_flattened() {
    let (d, mut e, id) = setup();
    fs::create_dir_all(d.path().join("College/AP/Prisma")).unwrap();
    fs::write(d.path().join("College/AP/Prisma/invoice.txt"), "synthetic").unwrap();
    stable(&d.path().join("College/AP/Prisma/invoice.txt"));
    let p = e.prepare_setup(id).unwrap();
    assert_eq!(p.inventory.files, 2);
    assert!(p.inventory.folders.contains(&"College/AP/Prisma".into()));
    assert_eq!(p.inventory.proposals.len(), 1);
    assert_eq!(
        p.inventory.proposals[0].destination.as_deref(),
        Some("College/lecture.txt")
    );
    assert!(p.create_folders.is_empty());
}
#[test]
fn ambiguous_existing_destinations_never_auto_move() {
    let (d, mut e, id) = setup();
    for name in ["College", "School"] {
        fs::create_dir(d.path().join(name)).unwrap()
    }
    let p = e.prepare_setup(id).unwrap();
    assert!(!p.inventory.proposals[0].certain);
    e.finish_setup(id, p.token).unwrap();
    let p = e.maintain(id).unwrap().proposals.remove(0);
    assert!(e.apply(p.id, false).is_err());
    assert!(e.apply(p.id, true).is_ok());
}
#[test]
fn project_or_symlink_destination_is_not_reused() {
    let (d, mut e, id) = setup();
    fs::create_dir(d.path().join("College")).unwrap();
    fs::write(d.path().join("College/package.json"), "{}").unwrap();
    std::os::unix::fs::symlink(d.path().join("College"), d.path().join("School")).unwrap();
    let p = e.prepare_setup(id).unwrap();
    assert_eq!(p.create_folders, vec!["Education"]);
    assert_eq!(p.inventory.protected, 2);
}
#[test]
fn home_style_inboxes_feed_existing_structure_but_collections_stay() {
    let (d, mut e, id) = setup();
    fs::create_dir(d.path().join("Downloads")).unwrap();
    fs::rename(
        d.path().join("lecture.txt"),
        d.path().join("Downloads/lecture.txt"),
    )
    .unwrap();
    fs::create_dir(d.path().join("College")).unwrap();
    let p = e.prepare_setup(id).unwrap();
    assert_eq!(p.inventory.proposals[0].source, "Downloads/lecture.txt");
    assert_eq!(
        p.inventory.proposals[0].destination.as_deref(),
        Some("College/lecture.txt")
    );
}
#[test]
fn ongoing_full_maintenance_can_prepare_new_needed_category() {
    let (d, mut e, id) = setup();
    let p = e.prepare_setup(id).unwrap();
    e.finish_setup(id, p.token).unwrap();
    fs::write(d.path().join("receipt.txt"), "synthetic").unwrap();
    stable(&d.path().join("receipt.txt"));
    let inventory = e.maintain(id).unwrap();
    assert!(d.path().join("Finance").exists());
    let p = inventory
        .proposals
        .iter()
        .find(|p| p.source == "receipt.txt")
        .unwrap();
    assert!(!p.blocked);
    assert!(d.path().join("receipt.txt").exists());
}
#[test]
fn no_setup_smart_and_paused_modes_do_not_create_folders() {
    for scenario in 0..3 {
        let (d, mut e, id) = setup();
        if scenario > 0 {
            e.mark_initial_reviewed(id).unwrap();
            e.set_settings(Settings {
                mode: if scenario == 1 {
                    Mode::Smart
                } else {
                    Mode::Full
                },
                monitoring: scenario != 2,
                ..e.settings.clone()
            })
            .unwrap()
        }
        e.maintain(id).unwrap();
        assert!(!d.path().join("Education").exists());
    }
}
#[test]
fn live_inbox_drops_applied_expired_and_revoked_actions() {
    let (_d, mut e, id) = setup();
    let p = e.prepare_setup(id).unwrap();
    e.finish_setup(id, p.token).unwrap();
    let p = e.maintain(id).unwrap().proposals.remove(0);
    e.apply(p.id, false).unwrap();
    assert!(e.inventories()[0].proposals.is_empty());
    e.set_settings(e.settings.clone()).unwrap();
    assert!(e.inventories()[0].proposals.is_empty());
    e.revoke(id).unwrap();
    assert!(e.inventories().is_empty());
}
#[test]
fn structure_preference_persists_and_expires_preview() {
    let (d, mut e, id) = setup();
    fs::create_dir(d.path().join("College")).unwrap();
    let p = e.prepare_setup(id).unwrap();
    e.set_preferences(Preferences {
        reuse_existing: false,
    })
    .unwrap();
    assert!(e.finish_setup(id, p.token).is_err());
    let p = e.prepare_setup(id).unwrap();
    assert_eq!(p.create_folders, vec!["Education"]);
    let db = d.path().join("preferences.sqlite");
    let mut e = Engine::new(Connection::open(&db).unwrap()).unwrap();
    e.set_preferences(Preferences {
        reuse_existing: false,
    })
    .unwrap();
    drop(e);
    let e = Engine::new(Connection::open(&db).unwrap()).unwrap();
    assert!(!e.preferences().unwrap().reuse_existing);
}
#[test]
fn preview_token_is_required_even_for_empty_folders() {
    let (_d, mut e, id) = setup();
    assert!(e.finish_setup(id, 1).is_err());
    let p = e.prepare_setup(id).unwrap();
    assert!(e.finish_setup(id, p.token + 1).is_err());
}
#[test]
fn permission_cancellation_prevents_setup_and_automatic_directory_creation() {
    let (d, mut e, id) = setup();
    let p = e.prepare_setup(id).unwrap();
    assert!(e.finish_setup_guarded(id, p.token, || false).is_err());
    assert!(!d.path().join("Education").exists());
    assert!(!e.roots[&id].initial_reviewed);
    e.finish_setup(id, p.token).unwrap();
    fs::write(d.path().join("receipt.txt"), "synthetic").unwrap();
    stable(&d.path().join("receipt.txt"));
    assert!(e.maintain_guarded(id, || false).is_err());
    assert!(!d.path().join("Finance").exists());
}
#[test]
fn hidden_app_and_system_collections_are_not_traversed() {
    let (d, mut e, id) = setup();
    for name in [".config", "Library", "Applications", "System"] {
        fs::create_dir(d.path().join(name)).unwrap();
        fs::write(d.path().join(name).join("lecture.txt"), "sensitive").unwrap();
    }
    let p = e.prepare_setup(id).unwrap();
    assert_eq!(p.inventory.files, 1);
    assert_eq!(p.inventory.protected, 4);
    assert!(p.inventory.folders.is_empty());
}
