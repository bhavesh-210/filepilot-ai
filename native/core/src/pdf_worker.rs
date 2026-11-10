use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
/// The helper must be an installed trusted application resource rather than a discovered file
pub fn inspect(helper: &Path, payload: Vec<u8>) -> Result<serde_json::Value, String> {
    if payload.len() > 2097152 {
        return Err("PDF input limit".into());
    }
    use std::os::unix::process::CommandExt;
    let mut child = Command::new(helper)
        .process_group(0)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().ok_or("Parser stdin unavailable")?;
    let output = child.stdout.take().ok_or("Parser stdout unavailable")?;
    let writer = thread::spawn(move || input.write_all(&payload));
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        output.take(200001).read_to_end(&mut bytes).map(|_| bytes)
    });
    let start = Instant::now();
    let mut exited = None;
    let status = loop {
        if exited.is_none() {
            match child.try_wait() {
                Ok(Some(status)) => exited = Some(status),
                Ok(None) => {}
                Err(error) => break Err(error.to_string()),
            }
        }
        if let Some(status) = exited {
            if writer.is_finished() && reader.is_finished() {
                break Ok(status);
            }
        }
        if start.elapsed() > Duration::from_secs(5) {
            break Err("PDF parser timeout".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    if status.is_err() {
        unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = writer.join();
    let bytes = reader
        .join()
        .map_err(|_| "PDF reader failed")?
        .map_err(|e| e.to_string())?;
    let status = status?;
    if !status.success() || bytes.len() > 200000 {
        return Err("PDF parser failed safely".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid parser response")?;
    if value["status"] != "ok" && value["status"] != "no_text" {
        return Err(value["reason"]
            .as_str()
            .unwrap_or("PDF extraction skipped")
            .into());
    }
    let text = value["text"].as_str().ok_or("Parser text missing")?;
    if text.chars().count() > 2000 {
        return Err("Parser text limit exceeded".into());
    }
    Ok(value)
}
