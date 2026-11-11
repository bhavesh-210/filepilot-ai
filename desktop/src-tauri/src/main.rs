#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use filepilot_core::pdf_worker as pdf;
use filepilot_core::{Engine, Preferences, Settings};
use notify::{RecursiveMode, Watcher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    collections::HashSet,
    ffi::{CStr, CString},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
#[derive(Clone)]
struct Shared {
    engine: Arc<Mutex<Engine>>,
    notice: Arc<Mutex<String>>,
    epoch: Arc<AtomicU64>,
}
#[cfg(target_os = "macos")]
extern "C" {
    fn fp_request_notifications() -> *mut libc::c_char;
    fn fp_notify_review(count: i32);
    fn fp_confirm_trash(source: *const libc::c_char) -> *mut libc::c_char;
    fn fp_semantic_rank(input: *const libc::c_char) -> *mut libc::c_char;
    fn fp_choose_roots() -> *mut libc::c_char;
    fn fp_resolve_root(encoded: *const libc::c_char) -> *mut libc::c_char;
    fn fp_release_root(path: *const libc::c_char);
    fn fp_login(enabled: bool) -> *mut libc::c_char;
}
#[cfg(target_os = "macos")]
unsafe fn native_json(pointer: *mut libc::c_char) -> Result<serde_json::Value, String> {
    if pointer.is_null() {
        return Err("Native integration returned no result".into());
    }
    let raw = CStr::from_ptr(pointer).to_string_lossy().into_owned();
    libc::free(pointer.cast());
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|_| "Invalid native response")?;
    if let Some(error) = value["error"].as_str() {
        return Err(error.into());
    }
    Ok(value)
}
#[tauri::command]
fn cleanup_profile(
    state: tauri::State<Shared>,
) -> Result<Option<filepilot_core::CleanupProfile>, String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .saved_cleanup_profile()
}
#[tauri::command]
fn save_cleanup_profile(token: u64, state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut e = state.engine.lock().map_err(|_| "Engine lock failed")?;
    e.save_cleanup_profile(token)?;
    e.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
fn clear_cleanup_profile(state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut e = state.engine.lock().map_err(|_| "Engine lock failed")?;
    e.clear_cleanup_profile()?;
    e.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
fn cleanup_runs(state: tauri::State<Shared>) -> Result<Vec<filepilot_core::CleanupRun>, String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .cleanup_runs()
}
#[tauri::command]
fn cancel_cleanup(token: u64, state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .cancel_cleanup(token);
    Ok(())
}
#[tauri::command]
async fn prepare_cleanup(
    profile: filepilot_core::CleanupProfile,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::CleanupPlan, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .prepare_cleanup(profile)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn apply_cleanup(
    save_profile: bool,
    token: u64,
    choices: Vec<(usize, String)>,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::CleanupResult, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut e = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = e.permission_epoch;
        e.apply_cleanup_with_profile(token, choices, save_profile, || {
            epoch.load(Ordering::SeqCst) == expected
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn undo_cleanup(
    run_id: i64,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::CleanupResult, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut e = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = e.permission_epoch;
        e.undo_cleanup(run_id, || epoch.load(Ordering::SeqCst) == expected)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn snapshot(state: tauri::State<Shared>) -> Result<serde_json::Value, String> {
    let engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    let roots=engine.saved_roots()?.iter().map(|r|serde_json::json!({"id":r.id,"path":r.path,"initial_reviewed":r.initial_reviewed,"active":engine.roots.contains_key(&r.id)})).collect::<Vec<_>>();
    Ok(
        serde_json::json!({"directories":engine.directory_stats()?,"routes":engine.routes()?,"transfers":engine.transfer_plans(),"preferences":engine.preferences()?,"inventories":engine.inventories(),"settings":engine.settings,"roots":roots,"history":engine.history()?,"notice":*state.notice.lock().map_err(|_|"Notice lock failed")?}),
    )
}
#[tauri::command]
async fn choose_folders(
    app: tauri::AppHandle,
    state: tauri::State<'_, Shared>,
) -> Result<serde_json::Value, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    let (send, recv) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        let result = (|| {
            let value = unsafe { native_json(fp_choose_roots()) }?;
            let mut ids = Vec::new();
            let mut errors = Vec::new();
            if let Some(items) = value["folders"].as_array() {
                let token = epoch.fetch_add(1, Ordering::SeqCst) + 1;
                let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
                engine.permission_epoch = token;
                for item in items {
                    let path = item["path"].as_str().ok_or("Missing folder path")?;
                    let bookmark = item["bookmark"].as_str().ok_or("Missing bookmark")?;
                    if bookmark.is_empty() { errors.push(format!("{path} Could not retain folder access")); continue; }
                    match engine.authorize(PathBuf::from(path), bookmark.into()) {
                        Ok(id) => ids.push(id),
                        Err(error) => {
                            if let Ok(path) = CString::new(path) { unsafe { fp_release_root(path.as_ptr()) }; }
                            errors.push(format!("{path} {error}"));
                        }
                    }
                }
            }
            Ok(serde_json::json!({"ids":ids,"errors":errors,"cancelled":value["cancelled"] == true}))
        })();
        #[cfg(not(target_os = "macos"))]
        let result: Result<serde_json::Value, String> = Err("macOS required".into());
        let _ = send.send(result);
    }).map_err(|e|e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        recv.recv().map_err(|_| "Folder picker ended".to_string())?
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn update_preferences(preferences: Preferences, state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.permission_epoch = epoch;
    engine.set_preferences(preferences)
}
#[tauri::command]
async fn prepare_setup(
    root_id: i64,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::SetupPreview, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .prepare_setup(root_id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn finish_setup(
    root_id: i64,
    token: u64,
    state: tauri::State<'_, Shared>,
) -> Result<(), String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = engine.permission_epoch;
        engine.finish_setup_guarded(root_id, token, || epoch.load(Ordering::SeqCst) == expected)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn collection_title(
    root_id: i64,
    sources: Vec<String>,
    state: tauri::State<Shared>,
) -> Result<String, String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .collection_title(root_id, &sources)
}
#[tauri::command]
fn plan_collection(
    root_id: i64,
    sources: Vec<String>,
    folder: String,
    create_folder: bool,
    state: tauri::State<Shared>,
) -> Result<filepilot_core::CollectionPlan, String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.permission_epoch = epoch;
    engine.plan_collection(root_id, sources, folder, create_folder)
}
#[tauri::command]
fn cancel_collection(root_id: i64, token: u64, state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .cancel_collection(root_id, token)
}
#[tauri::command]
async fn apply_collection(
    root_id: i64,
    token: u64,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::CollectionResult, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = engine.permission_epoch;
        engine.apply_collection(root_id, token, || epoch.load(Ordering::SeqCst) == expected)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn scan_all(app: tauri::AppHandle, state: tauri::State<'_, Shared>) -> Result<(), String> {
    let engine = state.engine.clone();
    let helper = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("filepilot-pdf");
    let notice = state.notice.clone();
    let shared_engine = engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let ids = engine.roots.keys().copied().collect::<Vec<_>>();
        let mut errors = Vec::new();
        for id in ids {
            if let Err(error) = engine.scan_background(id) {
                errors.push(format!("Location {id} needs attention {error}"));
            }
        }
        *notice.lock().map_err(|_| "Notice lock failed")? = errors.join("\n");
        drop(engine);
        plan_workspace(&shared_engine, &helper)?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn save_route(
    route: filepilot_core::Route,
    create: bool,
    state: tauri::State<Shared>,
) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.set_route(route, create, || {
        state.epoch.load(Ordering::SeqCst) == epoch
    })?;
    engine.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
fn clear_routes(state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.clear_routes()?;
    engine.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
fn prepare_transfer(
    source_root_id: i64,
    source: String,
    target_root_id: i64,
    folder: String,
    state: tauri::State<Shared>,
) -> Result<filepilot_core::TransferPlan, String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .prepare_transfer(source_root_id, source, target_root_id, folder)
}
#[tauri::command]
async fn apply_transfer(token: u64, state: tauri::State<'_, Shared>) -> Result<i64, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = engine.permission_epoch;
        engine.apply_transfer(token, true, || epoch.load(Ordering::SeqCst) == expected)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn prepare_folder_transfer(
    source_root_id: i64,
    source: String,
    target_root_id: i64,
    folder: String,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::FolderTransferPlan, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let result = engine.prepare_folder_transfer(source_root_id, source, target_root_id, folder);
        engine.permission_epoch = epoch;
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn cancel_folder_transfer(token: u64, state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .cancel_folder_transfer(token);
    Ok(())
}
#[tauri::command]
async fn apply_folder_transfer(token: u64, state: tauri::State<'_, Shared>) -> Result<i64, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = engine.permission_epoch;
        engine.apply_folder_transfer(token, || epoch.load(Ordering::SeqCst) == expected)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn scan(
    root_id: i64,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::Inventory, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .scan(root_id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn remember_example(proposal_id: u64, state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.remember_example(proposal_id)?;
    engine.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
async fn trash_file(
    root_id: i64,
    relative_path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Shared>,
) -> Result<Option<i64>, String> {
    let prepared = state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .prepare_trash(root_id, relative_path)?;
    let full_path = PathBuf::from(&prepared.root_path)
        .join(&prepared.source)
        .to_str()
        .ok_or("Unsupported path")?
        .to_string();
    let (send, recv) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        let result = (|| unsafe {
            native_json(fp_confirm_trash(
                CString::new(full_path)
                    .map_err(|_| "Invalid source path")?
                    .as_ptr(),
            ))
        })();
        #[cfg(not(target_os = "macos"))]
        let result: Result<serde_json::Value, String> =
            Err("macOS Trash confirmation required".into());
        let _ = send.send(result);
    })
    .map_err(|e| e.to_string())?;
    let response = tauri::async_runtime::spawn_blocking(move || {
        recv.recv()
            .map_err(|_| "Trash confirmation ended".to_string())?
    })
    .await
    .map_err(|e| e.to_string())??;
    if response["cancelled"] == true {
        return Ok(None);
    }
    if response["confirmed"] != true {
        return Err("Separate Trash confirmation required".into());
    }
    let path = PathBuf::from(
        response["trash_root"]
            .as_str()
            .ok_or("System Trash root missing")?,
    );
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    let expected = prepared.epoch;
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .trash_guarded(prepared, path, true, || {
                epoch.load(Ordering::SeqCst) == expected
            })
            .map(Some)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn reconcile(state: tauri::State<'_, Shared>) -> Result<(), String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine.lock().map_err(|_| "Engine lock failed")?.recover()
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn summarize_file(
    proposal_id: u64,
    app: tauri::AppHandle,
    state: tauri::State<'_, Shared>,
) -> Result<filepilot_core::FileSummary, String> {
    let engine = state.engine.clone();
    let helper = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("filepilot-pdf");
    tauri::async_runtime::spawn_blocking(move || {
        let payload = engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .pdf_snapshot(proposal_id);
        let parsed = match payload {
            Ok(Some(bytes)) => Some(
                pdf::inspect(&helper, bytes)
                    .unwrap_or_else(|error| serde_json::json!({"status":"skipped","reason":error})),
            ),
            Ok(None) => None,
            Err(error) => Some(serde_json::json!({"status":"skipped","reason":error})),
        };
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .summary(proposal_id, parsed.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn keep_here(proposal_id: u64, state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.permission_epoch = epoch;
    engine.keep_here(proposal_id)
}
#[tauri::command]
fn clear_kept(state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .clear_kept()
}
#[tauri::command]
fn review_folder(
    proposal_id: u64,
    folder: String,
    state: tauri::State<Shared>,
) -> Result<filepilot_core::Proposal, String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.permission_epoch = epoch;
    engine.review_folder(proposal_id, folder)
}
#[tauri::command]
async fn request_notifications(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let (send, recv) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        let result = unsafe { native_json(fp_request_notifications()) };
        #[cfg(not(target_os = "macos"))]
        let result: Result<serde_json::Value, String> = Err("macOS required".into());
        let _ = send.send(result);
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        recv.recv()
            .map_err(|_| "Notification request ended".to_string())?
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn semantic_suggestions(
    proposal_id: u64,
    app: tauri::AppHandle,
    state: tauri::State<'_, Shared>,
) -> Result<serde_json::Value, String> {
    let engine = state.engine.clone();
    let helper = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("filepilot-pdf");
    tauri::async_runtime::spawn_blocking(move||{
  let (pdf_payload,text,content_used)={let engine=engine.lock().map_err(|_|"Engine lock failed")?;
   let payload=engine.pdf_snapshot(proposal_id)?;
   if payload.is_some(){(payload,String::new(),true)}else{let(text,used)=engine.analysis_text(proposal_id)?;(None,text,used)}
  };
  let parsed=pdf_payload.map(|payload|pdf::inspect(&helper,payload)).transpose()?;
  let text=parsed.as_ref().and_then(|value|value["text"].as_str()).unwrap_or(&text);
  #[cfg(target_os="macos")]let mut result=unsafe{native_json(fp_semantic_rank(CString::new(text).map_err(|_|"Invalid text")?.as_ptr()))}?;
  #[cfg(not(target_os="macos"))]let mut result=serde_json::json!({"available":false,"reason":"macOS embedding required"});
  result["content_used"]=content_used.into();
  if let Some(parsed)=parsed{result["pdf_inspection"]=serde_json::json!({"pages_inspected":parsed["pages_inspected"],"truncated":parsed["truncated"],"ocr":false});}
  Ok(result)
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn review_destination(
    proposal_id: u64,
    category: String,
    state: tauri::State<Shared>,
) -> Result<filepilot_core::Proposal, String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    let proposal = engine.review_destination(proposal_id, category)?;
    engine.permission_epoch = epoch;
    Ok(proposal)
}
#[tauri::command]
async fn duplicates(
    root_id: i64,
    state: tauri::State<'_, Shared>,
) -> Result<serde_json::Value, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .duplicates(root_id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn update_settings(settings: Settings, state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.set_settings(settings)?;
    engine.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
async fn apply(proposal_id: u64, state: tauri::State<'_, Shared>) -> Result<i64, String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = engine.permission_epoch;
        engine.apply_guarded(proposal_id, true, || {
            epoch.load(Ordering::SeqCst) == expected
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn undo(operation_id: i64, state: tauri::State<'_, Shared>) -> Result<(), String> {
    let engine = state.engine.clone();
    let epoch = state.epoch.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine.lock().map_err(|_| "Engine lock failed")?;
        let expected = engine.permission_epoch;
        engine.undo_guarded(operation_id, || epoch.load(Ordering::SeqCst) == expected)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn correction(
    root_id: i64,
    category: String,
    destination: String,
    rule: bool,
    state: tauri::State<Shared>,
) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.correct(root_id, category, destination, rule)?;
    engine.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
fn clear_learning(state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.clear_learning()?;
    engine.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
fn clear_rules(state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    engine.clear_rules()?;
    engine.permission_epoch = epoch;
    Ok(())
}
#[tauri::command]
fn revoke(root_id: i64, app: tauri::AppHandle, state: tauri::State<Shared>) -> Result<(), String> {
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    let path = engine.roots.get(&root_id).map(|r| r.path.clone());
    engine.revoke(root_id)?;
    engine.permission_epoch = epoch;
    if let Some(path) = path {
        app.run_on_main_thread(move || {
            #[cfg(target_os = "macos")]
            if let Ok(path) = CString::new(path) {
                unsafe { fp_release_root(path.as_ptr()) };
            }
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
async fn login_item(enabled: bool, app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let (send, recv) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        let result = unsafe { native_json(fp_login(enabled)) };
        #[cfg(not(target_os = "macos"))]
        let result: Result<serde_json::Value, String> = Err("macOS required".into());
        let _ = send.send(result);
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        recv.recv()
            .map_err(|_| "Login item request ended".to_string())?
    })
    .await
    .map_err(|e| e.to_string())?
}
fn plan_workspace(
    engine: &Arc<Mutex<Engine>>,
    helper: &std::path::Path,
) -> Result<Vec<filepilot_core::TransferPlan>, String> {
    let mut plans = engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .plan_routes()?;
    let candidates = engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .pdf_route_candidates()?;
    for id in candidates {
        let payload = engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .pdf_snapshot(id);
        if let Ok(Some(payload)) = payload {
            let parsed = pdf::inspect(helper, payload)
                .unwrap_or_else(|error| serde_json::json!({"status":"skipped","reason":error}));
            if let Ok(Some(plan)) = engine
                .lock()
                .map_err(|_| "Engine lock failed")?
                .plan_pdf_route(id, &parsed)
            {
                plans.push(plan);
            }
        }
    }
    for plan in engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .transfer_plans()
    {
        if plan.automatic && !plans.iter().any(|p| p.token == plan.token) && plans.len() < 26 {
            plans.push(plan);
        }
    }
    Ok(plans)
}
fn monitor(app: tauri::AppHandle, state: Shared) {
    std::thread::spawn(move || {
        let (send, recv) = std::sync::mpsc::sync_channel(1);
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let _ = send.try_send(event);
            })
            .ok();
        let helper = app
            .path()
            .resource_dir()
            .ok()
            .map(|path| path.join("filepilot-pdf"));
        let mut watched = HashSet::<PathBuf>::new();
        let mut last = Instant::now() - Duration::from_secs(60);
        let mut notified = HashSet::<(i64, u64)>::new();
        let mut last_notification = Instant::now() - Duration::from_secs(1000);
        loop {
            let event = recv.recv_timeout(Duration::from_secs(5));
            if matches!(event, Err(std::sync::mpsc::RecvTimeoutError::Disconnected)) {
                std::thread::sleep(Duration::from_secs(5));
            }
            let dirty = match event {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => {
                    *state.notice.lock().unwrap()="Filesystem event overflow or error scheduled reconciliation remains active".into();
                    true
                }
                Err(_) => false,
            };
            let roots = {
                let engine = state.engine.lock().unwrap();
                if engine.settings.monitoring {
                    engine.roots.values().cloned().collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            };
            let wanted = roots
                .iter()
                .map(|r| PathBuf::from(&r.path))
                .collect::<HashSet<_>>();
            if let Some(w) = watcher.as_mut() {
                for old in watched.difference(&wanted) {
                    let _ = w.unwatch(old);
                }
                for new in wanted.difference(&watched) {
                    if let Err(e) = w.watch(new, RecursiveMode::Recursive) {
                        *state.notice.lock().unwrap() =
                            format!("Watcher unavailable scheduled reconciliation active {e}");
                    }
                }
            } else {
                *state.notice.lock().unwrap() =
                    "Event watcher unavailable scheduled reconciliation active".into();
            }
            watched = wanted;
            if !dirty && last.elapsed() < Duration::from_secs(30) {
                continue;
            }
            last = Instant::now();
            let mut applied = false;
            let mut applied_roots = HashSet::new();
            for root in roots {
                let scan = {
                    let mut engine = state.engine.lock().unwrap();
                    let expected = engine.permission_epoch;
                    engine.maintain_guarded(root.id, || {
                        state.epoch.load(Ordering::SeqCst) == expected
                    })
                };
                match scan {
                    Ok(inventory) => {
                        for p in inventory.proposals {
                            if !p.blocked && p.certain {
                                let mut engine = state.engine.lock().unwrap();
                                if !engine.settings.monitoring {
                                    break;
                                }
                                if (engine.folder_is_held(p.root_id, &p.source)
                                    || engine.cleanup_is_held(p.root_id)
                                    || engine.uses_cleanup_profile(p.root_id))
                                {
                                    continue;
                                }
                                if engine.route_for_proposal(&p).ok().flatten().is_some() {
                                    continue;
                                }
                                let expected = engine.permission_epoch;
                                match engine.apply_guarded(p.id, false, || {
                                    state.epoch.load(Ordering::SeqCst) == expected
                                }) {
                                    Ok(_) => {
                                        applied = true;
                                        applied_roots.insert(root.id);
                                    }
                                    Err(error) => {
                                        if !error.contains("approval") && !error.contains("rules") {
                                            *state.notice.lock().unwrap() =
                                                format!("Automatic action paused {error}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        *state.notice.lock().unwrap() = format!("Folder requires attention {e}");
                    }
                }
            }
            {
                let mut engine = state.engine.lock().unwrap();
                let expected = engine.permission_epoch;
                match engine
                    .maintain_cleanup_profile(|| state.epoch.load(Ordering::SeqCst) == expected)
                {
                    Ok(Some(result)) => {
                        if result.applied > 0 {
                            applied = true;
                            applied_roots.extend(roots.iter().map(|r| r.id));
                        }
                        if let Some(error) = result.error {
                            *state.notice.lock().unwrap() =
                                format!("Layout maintenance stopped {error}");
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        *state.notice.lock().unwrap() = format!("Layout needs attention {error}");
                    }
                }
            }
            let plans = if let Some(helper) = &helper {
                plan_workspace(&state.engine, helper)
            } else {
                state.engine.lock().unwrap().plan_routes()
            };
            if let Ok(plans) = plans {
                for plan in plans {
                    let mut engine = state.engine.lock().unwrap();
                    if engine.settings.mode == filepilot_core::Mode::Smart
                        || !engine.settings.monitoring
                    {
                        break;
                    }
                    if engine.folder_is_held(plan.source_root_id, &plan.source) {
                        continue;
                    }
                    let expected = engine.permission_epoch;
                    match engine.apply_transfer(plan.token, false, || {
                        state.epoch.load(Ordering::SeqCst) == expected
                    }) {
                        Ok(_) => {
                            applied = true;
                            applied_roots.insert(plan.source_root_id);
                            applied_roots.insert(plan.target_root_id);
                        }
                        Err(error) => {
                            *state.notice.lock().unwrap() =
                                format!("Routing needs attention {error}");
                        }
                    }
                }
            }
            if applied {
                let mut engine = state.engine.lock().unwrap();
                for root_id in &applied_roots {
                    let _ = engine.scan_background(*root_id);
                }
            }
            let (notifications, pending) = {
                let engine = state.engine.lock().unwrap();
                (
                    engine.settings.review_notifications,
                    engine
                        .inventories()
                        .iter()
                        .flat_map(|i| {
                            i.proposals
                                .iter()
                                .map(|p| (p.root_id, p.id))
                                .collect::<Vec<_>>()
                        })
                        .collect::<HashSet<_>>(),
                )
            };
            if notifications
                && !pending.is_empty()
                && !pending.is_subset(&notified)
                && last_notification.elapsed() >= Duration::from_secs(900)
            {
                #[cfg(target_os = "macos")]
                unsafe {
                    fp_notify_review(pending.len().min(i32::MAX as usize) as i32)
                };
                notified = pending;
                last_notification = Instant::now();
            }
            let _ = app.emit(
                "filepilot-updated",
                serde_json::json!({"applied":applied,"root_ids":applied_roots}),
            );
        }
    });
}
fn main() {
    let application = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            cleanup_profile,
            save_cleanup_profile,
            clear_cleanup_profile,
            cleanup_runs,
            cancel_cleanup,
            prepare_cleanup,
            apply_cleanup,
            undo_cleanup,
            prepare_folder_transfer,
            cancel_folder_transfer,
            apply_folder_transfer,
            scan_all,
            save_route,
            clear_routes,
            prepare_transfer,
            apply_transfer,
            summarize_file,
            keep_here,
            clear_kept,
            review_folder,
            request_notifications,
            collection_title,
            plan_collection,
            cancel_collection,
            apply_collection,
            choose_folders,
            update_preferences,
            prepare_setup,
            finish_setup,
            remember_example,
            trash_file,
            reconcile,
            semantic_suggestions,
            review_destination,
            duplicates,
            snapshot,
            scan,
            update_settings,
            apply,
            undo,
            correction,
            clear_learning,
            clear_rules,
            revoke,
            login_item
        ])
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o700))?;
            }
            let mut engine =
                Engine::new(rusqlite::Connection::open(data.join("filepilot.sqlite"))?)
                    .map_err(std::io::Error::other)?;
            for saved in engine.saved_roots().map_err(std::io::Error::other)? {
                #[cfg(target_os = "macos")]
                if let Ok(bookmark) = CString::new(saved.bookmark.clone()) {
                    if let Ok(value) = unsafe { native_json(fp_resolve_root(bookmark.as_ptr())) } {
                        if value["path"].as_str() == Some(&saved.path) {
                            let _ = engine.restore(PathBuf::from(&saved.path), saved.bookmark);
                        }
                    }
                }
            }
            engine.recover().map_err(std::io::Error::other)?;
            let state = Shared {
                engine: Arc::new(Mutex::new(engine)),
                notice: Arc::new(Mutex::new(String::new())),
                epoch: Arc::new(AtomicU64::new(0)),
            };
            app.manage(state.clone());
            monitor(app.handle().clone(), state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("FilePilot could not start");
    application.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });
}
