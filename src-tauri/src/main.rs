// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{collections::HashMap, process::Stdio, sync::Arc};
use reqwest::{Client, Proxy};
use serde_json::json;
use tauri::http::{HeaderMap, HeaderName, HeaderValue};
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
        .invoke_handler(tauri::generate_handler![toggle_intercept, send_request])
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
        .arg("--set").arg("stream_large_bodies=1") 
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
async fn send_request(app: AppHandle, raw: String, id: usize) {
    let proxy = Proxy::all("localhost:3009").unwrap();
    let client = Client::builder()
        .proxy(proxy).build().unwrap();

    let raw_owned = raw.clone();

    if let Some((headers_str, body)) = raw_owned.split_once("\r\n") {
        let mut headers = HeaderMap::new();

        // all elements become owned Strings, safe for insertion
        let headers_split: Vec<String> = headers_str
            .lines()
            .map(|line| line.to_string())
            .collect();

        for header in headers_split {
            if let Some((key, value)) = header.split_once(":") {
                let hname = HeaderName::from_bytes(key.trim().as_bytes()).unwrap();
                let hvalue = HeaderValue::from_str(value.trim()).unwrap();
                headers.insert(hname, hvalue);
            }
        }

        let res = client
            .post("http://httpbin.org/post")
            .headers(headers)
            .body(body.to_string())
            .send()
            .await;

        if let Ok(res) = res {
            #[derive(Serialize, Default, Debug)]
            struct Res {
                url: String,
                status: String,
                headers: HashMap<String, String>,
            }

            let mut response = Res::default();
            response.url = res.url().to_string();
            response.status = res.status().to_string();
            response.headers = res.headers().iter().map(|(key ,value)| (key.to_string(), value.to_str().unwrap().to_string())).collect();
            println!("Sending: {:?}", response);

            let _ = app.emit("forwarded-request-received", json!(response));
        }
    }
}

#[tauri::command]
fn toggle_intercept(state: State<'_, Arc<AppState>>, intercept_toggle: bool) {
    state.intercept.store(intercept_toggle, Ordering::Relaxed);
    println!("Intercept toggled: {}", intercept_toggle);
}

fn main() {
    run();
}