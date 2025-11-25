// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{collections::HashMap, process::Stdio, sync::Arc};
use tauri::{AppHandle, Emitter, State};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::sleep;

struct AppState {
    intercept: AtomicBool
}

#[derive(Clone, Serialize, Deserialize)]
struct AppRequest {
    method: String,
    uri: String,
    headers: HashMap<String, String>,
    body: String,
    raw: String,
    id: String
}

#[derive(Clone, Serialize, Deserialize)]
struct AppResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    raw: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState {
        intercept: AtomicBool::new(false)
    });
    let state_clone = state.clone();
    tauri::Builder::default()
        .setup(|app| {
            tauri::async_runtime::spawn(run_proxy(Arc::new(app.handle().clone()), state_clone));
            Ok(())
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![toggle_intercept])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn run_proxy(app_handle: Arc<AppHandle>, state: Arc<AppState>) {
    let root = env!("CARGO_MANIFEST_DIR");
    let script_path = format!("{}\\..\\scripts\\proxy.py", root);
    println!("{}", script_path);
    let mut proxy_child = Command::new("mitmdump")
        .arg("-p").arg("3009")
        .arg("-s").arg(&script_path)
        .arg("--set").arg("console_eventlog_verbosity=off")
        .arg("--set").arg("stream_large_bodies=1")  // Important!
        .arg("--no-http2")
        .arg("-q")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().expect("Failed to start mitmdump");

    let dump_out = proxy_child.stdout.take().unwrap();
    let dump_err = proxy_child.stderr.take().unwrap();

    let mut out_reader = BufReader::new(dump_out).lines();
    let mut err_reader = BufReader::new(dump_err).lines();
    let handle_clone = app_handle.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        loop {
            while let Ok(Some(line)) = out_reader.next_line().await {
                if state_clone.intercept.load(Ordering::Relaxed) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        if let Some(_) = json.get("method") {
                            let _ = handle_clone.clone().emit("request-received", json);
                        } else {
                            let _ = handle_clone.clone().emit("response-received", json);
                        }
                    }
                } else {
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    let handle_clone = app_handle.clone();
    let state_clone = state.clone();

    // Read stderr in its own task
    tokio::spawn(async move {
        loop {
            while let Ok(Some(line)) = err_reader.next_line().await {
                if state_clone.intercept.load(Ordering::Relaxed) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        if let Some(_) = json.get("method") {
                            let _ = handle_clone.clone().emit("request-received", json);
                        } else {
                            let _ = handle_clone.clone().emit("response-received", json);
                        }
                    }
                } else {
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    println!("Proxy running...");

    // IMPORTANT: Wait for process so it doesn’t get dropped
    let status = proxy_child.wait().await.unwrap();
    println!("[mitmdump exited] {status}");
}

#[tauri::command]
fn toggle_intercept(state: State<'_, Arc<AppState>>, intercept_toggle: bool) {
    state.intercept.store(intercept_toggle, Ordering::Relaxed);
    println!("Intercept toggled: {}", intercept_toggle);
}

fn main() {
    run();
}