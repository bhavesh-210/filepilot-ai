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
fn snapshot(state: tauri::State<Shared>) -> Result<serde_json::Value, String> {
    let engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    let roots=engine.saved_roots()?.iter().map(|r|serde_json::json!({"id":r.id,"path":r.path,"initial_reviewed":r.initial_reviewed,"active":engine.roots.contains_key(&r.id)})).collect::<Vec<_>>();
    Ok(
        serde_json::json!({"preferences":engine.preferences()?,"inventories":engine.inventories(),"settings":engine.settings,"roots":roots,"history":engine.history()?,"notice":*state.notice.lock().map_err(|_|"Notice lock failed")?}),
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
fn monitor(app: tauri::AppHandle, state: Shared) {
    std::thread::spawn(move || {
        let (send, recv) = std::sync::mpsc::sync_channel(1);
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let _ = send.try_send(event);
            })
            .ok();
        let mut watched = HashSet::<PathBuf>::new();
        let mut last = Instant::now() - Duration::from_secs(60);
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
                            if !p.blocked {
                                let mut engine = state.engine.lock().unwrap();
                                if !engine.settings.monitoring {
                                    break;
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
