static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
use filepilot_core::pdf_worker::inspect;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};
fn helper(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("trusted-test-parser");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    (temp, path)
}
#[test]
fn valid_bounded_response() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (_temp, path) =
        helper("printf '%s' '{\"status\":\"ok\",\"text\":\"synthetic\",\"pages_inspected\":1}'");
    let result = inspect(&path, b"%PDF-synthetic".to_vec()).unwrap();
    assert_eq!(result["text"], "synthetic");
}
#[test]
fn missing_and_invalid_parser_fail_closed() {
    let _lock = TEST_LOCK.lock().unwrap();
    assert!(inspect(
        std::path::Path::new("/missing-filepilot-test-parser"),
        vec![]
    )
    .is_err());
    let (_temp, path) = helper("printf '%s' 'not JSON'");
    assert!(inspect(&path, b"%PDF-synthetic".to_vec()).is_err());
    let (_temp, path) =
        helper("printf '%s' '{\"status\":\"skipped\",\"reason\":\"Encrypted PDF\"}'");
    assert!(inspect(&path, b"%PDF-synthetic".to_vec()).is_err());
}
#[test]
fn input_and_text_output_limits() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (_temp, path) = helper("exit 0");
    assert!(inspect(&path, vec![0; 2097153]).is_err());
    let response = serde_json::json!({"status":"ok","text":"x".repeat(2001)}).to_string();
    let (_temp, path) = helper(&format!("printf '%s' '{response}'"));
    assert!(inspect(&path, b"%PDF-synthetic".to_vec()).is_err());
}
#[test]
fn timeout_kills_parser_process_group() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (_temp, path) = helper("sleep 20");
    let started = Instant::now();
    assert!(inspect(&path, b"%PDF-synthetic".to_vec()).is_err());
    assert!(started.elapsed() < Duration::from_secs(9));
}
#[test]
fn an_exiting_parent_cannot_leave_the_output_pipe_hanging() {
    let _lock = TEST_LOCK.lock().unwrap();
    let (_temp, path) = helper("sleep 20 &\nexit 0");
    let started = Instant::now();
    assert!(inspect(&path, b"%PDF-synthetic".to_vec()).is_err());
    assert!(started.elapsed() < Duration::from_secs(9));
}
