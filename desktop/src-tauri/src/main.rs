#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use filepilot_core::{Engine, Settings};
use notify::{RecursiveMode, Watcher};
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
}
#[cfg(target_os = "macos")]
extern "C" {
    fn fp_choose_root() -> *mut libc::c_char;
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
        serde_json::json!({"settings":engine.settings,"roots":roots,"history":engine.history()?,"notice":*state.notice.lock().map_err(|_|"Notice lock failed")?}),
    )
}
#[tauri::command]
async fn choose_folder(
    app: tauri::AppHandle,
    state: tauri::State<'_, Shared>,
) -> Result<Option<i64>, String> {
    let engine = state.engine.clone();
    let (send, recv) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        let result = (|| {
            let value = unsafe { native_json(fp_choose_root()) }?;
            if value["cancelled"] == true {
                return Ok(None);
            }
            let path = value["path"].as_str().ok_or("Missing folder path")?;
            let bookmark = value["bookmark"].as_str().ok_or("Missing bookmark")?;
            let id = engine
                .lock()
                .map_err(|_| "Engine lock failed")?
                .authorize(PathBuf::from(path), bookmark.into())?;
            Ok(Some(id))
        })();
        #[cfg(not(target_os = "macos"))]
        let result: Result<Option<i64>, String> =
            Err("The native app currently requires macOS".into());
        let _ = send.send(result);
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        recv.recv().map_err(|_| "Folder picker ended".to_string())?
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
fn create_categories(root_id: i64, state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .create_categories(root_id)
}
#[tauri::command]
fn update_settings(settings: Settings, state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .set_settings(settings)
}
#[tauri::command]
fn enable_after_preview(root_id: i64, state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .mark_initial_reviewed(root_id)
}
#[tauri::command]
async fn apply(proposal_id: u64, state: tauri::State<'_, Shared>) -> Result<i64, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .apply(proposal_id, true)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn undo(operation_id: i64, state: tauri::State<'_, Shared>) -> Result<(), String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .lock()
            .map_err(|_| "Engine lock failed")?
            .undo(operation_id)
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
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .correct(root_id, category, destination, rule)
}
#[tauri::command]
fn clear_learning(state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .clear_learning()
}
#[tauri::command]
fn clear_rules(state: tauri::State<Shared>) -> Result<(), String> {
    state
        .engine
        .lock()
        .map_err(|_| "Engine lock failed")?
        .clear_rules()
}
#[tauri::command]
fn revoke(root_id: i64, app: tauri::AppHandle, state: tauri::State<Shared>) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|_| "Engine lock failed")?;
    let path = engine.roots.get(&root_id).map(|r| r.path.clone());
    engine.revoke(root_id)?;
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
        let (send, recv) = std::sync::mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let _ = send.send(event);
            })
            .ok();
        let mut watched = HashSet::<PathBuf>::new();
        let mut last = Instant::now() - Duration::from_secs(60);
        loop {
            let event = recv.recv_timeout(Duration::from_secs(5));
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
                if !engine.settings.monitoring {
                    continue;
                }
                engine.roots.values().cloned().collect::<Vec<_>>()
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
            for root in roots {
                let scan = state.engine.lock().unwrap().scan(root.id);
                match scan {
                    Ok(inventory) => {
                        for p in inventory.proposals {
                            if !p.blocked {
                                let mut engine = state.engine.lock().unwrap();
                                if !engine.settings.monitoring {
                                    break;
                                }
                                let _ = engine.apply(p.id, false);
                            }
                        }
                    }
                    Err(e) => {
                        *state.notice.lock().unwrap() = format!("Folder requires attention {e}");
                    }
                }
            }
            let _ = app.emit("filepilot-updated", ());
        }
    });
}
fn main() {
    let application = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            snapshot,
            choose_folder,
            scan,
            create_categories,
            update_settings,
            enable_after_preview,
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
                            let _ = engine.authorize(PathBuf::from(&saved.path), saved.bookmark);
                        }
                    }
                }
            }
            engine.recover().map_err(std::io::Error::other)?;
            let state = Shared {
                engine: Arc::new(Mutex::new(engine)),
                notice: Arc::new(Mutex::new(String::new())),
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
