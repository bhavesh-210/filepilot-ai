use filepilot_core::{Engine, Mode, Route, Settings};
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
    let mut e = Engine::new(Connection::open(base.join("journal.sqlite")).unwrap()).unwrap();
    for name in ["Downloads", "Documents"] {
        fs::create_dir(base.join(name)).unwrap();
    }
    let a = e.authorize(base.join("Downloads"), "a".into()).unwrap();
    let b = e.authorize(base.join("Documents"), "b".into()).unwrap();
    e.mark_initial_reviewed(a).unwrap();
    e.mark_initial_reviewed(b).unwrap();
    write(
        &base.join("Downloads"),
        "Archive/scan001.txt",
        "Invoice payment amount and bank transaction",
    );
    (d, e, a, b)
}
#[test]
fn cross_location_preview_move_stats_and_undo() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .unwrap();
    assert!(d.path().join("Downloads/Archive/scan001.txt").exists());
    assert!(e.history().unwrap().is_empty());
    let op = e.apply_transfer(p.token, true, || true).unwrap();
    assert!(d.path().join("Documents/scan001.txt").exists());
    let h = e.history().unwrap();
    assert_eq!(h[0].destination_root_id, Some(b));
    assert_eq!(h[0].kind, "transfer");
    let stats = e.directory_stats().unwrap();
    assert_eq!(stats.iter().find(|s| s.root_id == a).unwrap().sent, 1);
    assert_eq!(stats.iter().find(|s| s.root_id == b).unwrap().received, 1);
    e.undo(op).unwrap();
    assert!(d.path().join("Downloads/Archive/scan001.txt").exists());
    assert!(e.scan(a).unwrap().proposals.is_empty());
}
#[test]
fn transfer_requires_both_live_permissions() {
    let (_d, mut e, a, b) = setup();
    let p = e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .unwrap();
    e.revoke(b).unwrap();
    assert!(e.apply_transfer(p.token, true, || true).is_err());
}
#[test]
fn changed_source_collision_and_stale_settings_are_rejected() {
    for scenario in 0..3 {
        let (d, mut e, a, b) = setup();
        let p = e
            .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
            .unwrap();
        match scenario {
            0 => write(
                &d.path().join("Downloads"),
                "Archive/scan001.txt",
                "changed",
            ),
            1 => write(&d.path().join("Documents"), "scan001.txt", "occupied"),
            _ => e.set_settings(e.settings.clone()).unwrap(),
        };
        assert!(e.apply_transfer(p.token, true, || true).is_err());
        assert!(d.path().join("Downloads/Archive/scan001.txt").exists());
    }
}
#[test]
fn cancellation_prevents_transfer_and_undo() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .unwrap();
    assert!(e.apply_transfer(p.token, true, || false).is_err());
    assert!(d.path().join("Downloads/Archive/scan001.txt").exists());
    let op = e.apply_transfer(p.token, true, || true).unwrap();
    assert!(e.undo_guarded(op, || false).is_err());
    assert!(d.path().join("Documents/scan001.txt").exists());
}
#[test]
fn changed_transferred_file_and_occupied_original_block_undo() {
    for changed in [true, false] {
        let (d, mut e, a, b) = setup();
        let p = e
            .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
            .unwrap();
        let op = e.apply_transfer(p.token, true, || true).unwrap();
        if changed {
            write(&d.path().join("Documents"), "scan001.txt", "changed")
        } else {
            write(
                &d.path().join("Downloads"),
                "Archive/scan001.txt",
                "occupied",
            )
        };
        assert!(e.undo(op).is_err());
    }
}
#[test]
fn revoked_destination_permission_blocks_undo() {
    let (_d, mut e, a, b) = setup();
    let p = e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .unwrap();
    let op = e.apply_transfer(p.token, true, || true).unwrap();
    e.revoke(b).unwrap();
    assert!(e.undo(op).is_err());
}
#[test]
fn protected_source_destination_and_paths_stay_protected() {
    let (d, mut e, a, b) = setup();
    write(&d.path().join("Documents"), "Code/package.json", "{}");
    write(&d.path().join("Downloads"), "Code/package.json", "{}");
    write(&d.path().join("Downloads"), "Code/scan001.txt", "protected");
    for folder in ["../outside", "Code", ".ssh"] {
        assert!(e
            .prepare_transfer(a, "Archive/scan001.txt".into(), b, folder.into())
            .is_err());
    }
    assert!(e
        .prepare_transfer(a, "Code/scan001.txt".into(), b, "".into())
        .is_err());
}
#[test]
fn automatic_content_routing_requires_a_saved_opt_in_and_complete_text() {
    let (d, mut e, a, b) = setup();
    e.scan(a).unwrap();
    e.scan(b).unwrap();
    assert!(e.plan_routes().unwrap().is_empty());
    e.set_route(
        Route {
            category: "Finance".into(),
            target_root_id: b,
            folder: "Finance".into(),
            content: false,
        },
        true,
        || true,
    )
    .unwrap();
    e.scan(a).unwrap();
    assert!(e.plan_routes().unwrap().is_empty());
    e.set_route(
        Route {
            category: "Finance".into(),
            target_root_id: b,
            folder: "Finance".into(),
            content: true,
        },
        false,
        || true,
    )
    .unwrap();
    e.scan(a).unwrap();
    let p = e.plan_routes().unwrap().remove(0);
    e.apply_transfer(p.token, false, || true).unwrap();
    assert!(d.path().join("Documents/Finance/scan001.txt").exists());
    assert!(e.scan(b).unwrap().proposals.is_empty());
}
#[test]
fn smart_mode_does_not_execute_saved_routes_automatically() {
    let (d, mut e, a, b) = setup();
    e.set_settings(Settings {
        mode: Mode::Smart,
        ..e.settings.clone()
    })
    .unwrap();
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
    let p = e.plan_routes().unwrap().remove(0);
    assert!(e.apply_transfer(p.token, false, || true).is_err());
    assert!(d.path().join("Downloads/Archive/scan001.txt").exists());
    assert!(e.apply_transfer(p.token, true, || true).is_ok());
}
#[test]
fn ambiguous_partial_and_unsupported_documents_do_not_gain_content_routing_permission() {
    let (d, mut e, a, b) = setup();
    write(
        &d.path().join("Downloads"),
        "Archive/ambiguous.txt",
        "Invoice bank payment amount with university course assignment exam homework",
    );
    write(
        &d.path().join("Downloads"),
        "Archive/long.txt",
        &"Invoice bank payment amount transaction ".repeat(100),
    );
    write(
        &d.path().join("Downloads"),
        "Archive/scan.pdf",
        "%PDF-unsupported for automatic parsing",
    );
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
    let plans = e.plan_routes().unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].source, "Archive/scan001.txt");
}
#[test]
fn source_and_destination_setup_are_required() {
    let (d, mut e, a, b) = setup();
    let other = d.path().join("Desktop");
    fs::create_dir(&other).unwrap();
    let c = e.authorize(other.canonicalize().unwrap(), "c".into()).unwrap();
    assert!(e
        .prepare_transfer(a, "Archive/scan001.txt".into(), c, "".into())
        .is_err());
    assert!(e
        .set_route(
            Route {
                category: "Finance".into(),
                target_root_id: c,
                folder: "".into(),
                content: true
            },
            false,
            || true
        )
        .is_err());
    assert!(e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .is_ok());
}
#[test]
fn destination_root_replacement_is_not_authorized() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .unwrap();
    fs::rename(
        d.path().join("Documents"),
        d.path().join("OriginalDocuments"),
    )
    .unwrap();
    fs::create_dir(d.path().join("Documents")).unwrap();
    assert!(e.apply_transfer(p.token, true, || true).is_err());
}
#[test]
fn full_mode_undo_is_not_immediately_redone() {
    let (d, mut e, a, _b) = setup();
    e.create_categories(a).unwrap();
    write(&d.path().join("Downloads"), "receipt.txt", "synthetic");
    let p = e
        .scan(a)
        .unwrap()
        .proposals
        .into_iter()
        .find(|p| p.source == "receipt.txt")
        .unwrap();
    let op = e.apply(p.id, false).unwrap();
    e.undo(op).unwrap();
    assert!(!e
        .scan(a)
        .unwrap()
        .proposals
        .iter()
        .any(|p| p.source == "receipt.txt"));
}
#[test]
fn interrupted_transfer_recovers_without_overwrite() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .unwrap();
    let op = e.apply_transfer(p.token, true, || true).unwrap();
    let db = Connection::open(d.path().join("journal.sqlite")).unwrap();
    db.execute("UPDATE operations SET state='pending' WHERE id=?", [op])
        .unwrap();
    e.recover().unwrap();
    assert_eq!(e.history().unwrap()[0].state, "applied");
    e.undo(op).unwrap();
    assert!(d.path().join("Downloads/Archive/scan001.txt").exists());
}
#[test]
fn recovery_waits_for_both_permissions() {
    let (d, mut e, a, b) = setup();
    let p = e
        .prepare_transfer(a, "Archive/scan001.txt".into(), b, "".into())
        .unwrap();
    let op = e.apply_transfer(p.token, true, || true).unwrap();
    let db = Connection::open(d.path().join("journal.sqlite")).unwrap();
    db.execute("UPDATE operations SET state='pending' WHERE id=?", [op])
        .unwrap();
    e.revoke(b).unwrap();
    e.recover().unwrap();
    assert_eq!(e.history().unwrap()[0].state, "pending");
    e.authorize(
        d.path().join("Documents").canonicalize().unwrap(),
        "renewed".into(),
    )
    .unwrap();
    e.recover().unwrap();
    assert_eq!(e.history().unwrap()[0].state, "applied");
}
#[test]
fn existing_nested_destination_collection_stays_intact() {
    let (d, mut e, a, b) = setup();
    write(
        &d.path().join("Documents"),
        "Finance/2024/unknown.txt",
        "Invoice bank payment amount transaction",
    );
    e.set_route(
        Route {
            category: "Finance".into(),
            target_root_id: b,
            folder: "Finance".into(),
            content: true,
        },
        false,
        || true,
    )
    .unwrap();
    e.scan(a).unwrap();
    e.scan(b).unwrap();
    assert_eq!(e.plan_routes().unwrap().len(), 1);
    assert!(e.scan(b).unwrap().proposals.is_empty());
    assert!(d.path().join("Documents/Finance/2024/unknown.txt").exists());
    assert!(!d.path().join("Documents/Finance/unknown.txt").exists());
}
#[test]
fn complete_pdf_rule_can_route_but_requires_prior_content_permission() {
    let (d, mut e, a, b) = setup();
    write(
        &d.path().join("Downloads"),
        "Archive/unknown.pdf",
        "%PDF-synthetic source only",
    );
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
    let i = e.scan(a).unwrap();
    let p = i
        .proposals
        .iter()
        .find(|p| p.source.ends_with(".pdf"))
        .unwrap();
    let parsed = serde_json::json!({"status":"ok","truncated":false,"pages_inspected":2,"text":"Invoice bank payment amount transaction receipt"});
    let plan = e.plan_pdf_route(p.id, &parsed).unwrap().unwrap();
    e.apply_transfer(plan.token, false, || true).unwrap();
    assert!(d.path().join("Documents/Finance/unknown.pdf").exists());
}
#[test]
fn partial_pdf_and_competing_topics_never_get_automatic_pdf_routes() {
    for partial in [true, false] {
        let (d, mut e, a, b) = setup();
        write(
            &d.path().join("Downloads"),
            "Archive/unknown.pdf",
            "%PDF-synthetic",
        );
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
        let p = e
            .scan(a)
            .unwrap()
            .proposals
            .into_iter()
            .find(|p| p.source.ends_with(".pdf"))
            .unwrap();
        let text = if partial {
            "Invoice bank payment amount"
        } else {
            "Invoice bank payment amount university assignment course homework exam"
        };
        let parsed =
            serde_json::json!({"status":"ok","truncated":partial,"pages_inspected":2,"text":text});
        assert!(e.plan_pdf_route(p.id, &parsed).unwrap().is_none());
        assert!(d.path().join("Downloads/Archive/unknown.pdf").exists());
        assert!(e.pdf_route_candidates().unwrap().is_empty());
    }
}
