// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::crypto::sign;
use jsonwebtoken::*;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{collections::HashMap, process::Stdio, sync::Arc};
use tauri::http::{HeaderMap, HeaderName, HeaderValue};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::time::sleep;

struct AppState {
    intercept: AtomicBool,
}

#[derive(Clone, Serialize, Deserialize)]
struct AppRequest {
    method: String,
    uri: String,
    headers: HashMap<String, String>,
    body: String,
    raw: String,
    id: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct AppResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    raw: String,
}

#[derive(Serialize, Default, Debug)]
struct Res {
    url: String,
    status: String,
    headers: HashMap<String, String>,
    body: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState {
        intercept: AtomicBool::new(false),
    });
    let state_clone = state.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            tauri::async_runtime::spawn(run_proxy(Arc::new(app.handle().clone()), state_clone));
            Ok(())
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            toggle_intercept,
            send_request,
            parse_jwt_token,
            encode_jwt,
            probe_dirs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn run_proxy(app_handle: Arc<AppHandle>, state: Arc<AppState>) {
    let root = env!("CARGO_MANIFEST_DIR");
    let script_path = format!("{}\\..\\scripts\\proxy.py", root);
    println!("{}", script_path);
    let mut proxy_child = Command::new("mitmdump")
        .arg("-p")
        .arg("3009")
        .arg("-s")
        .arg(&script_path)
        .arg("--set")
        .arg("console_eventlog_verbosity=off")
        .arg("--set")
        .arg("stream_large_bodies=1")
        .arg("--no-http2")
        .arg("-q")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start mitmdump");

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
async fn send_request(app: AppHandle, raw: String) {
    let client = Client::new();
    let raw_owned = raw.clone();

    if let Some((headers_str, body)) = raw_owned.split_once("\r\n\r\n") {
        let mut headers = HeaderMap::new();

        let headers_split: Vec<String> = headers_str
            .lines()
            .skip(1)
            .map(|line| line.to_string())
            .collect();

        let request_line = headers_str.lines().next().unwrap();
        let (method, path, _version) =
            match request_line.split_whitespace().collect::<Vec<&str>>()[..] {
                [m, p, v] => (m, p, v),
                _ => {
                    eprintln!("Invalid request line: {}", request_line);
                    return;
                }
            };

        let mut host = None;
        for header in headers_split {
            if let Some((key, value)) = header.split_once(":") {
                if key.to_lowercase() == "host" {
                    host = Some(value.trim().to_string());
                }

                let hname = HeaderName::from_bytes(key.trim().as_bytes()).unwrap();
                let hvalue = HeaderValue::from_str(value.trim()).unwrap();
                headers.insert(hname, hvalue);
            }
        }

        headers.remove("accept-encoding");

        let url = match host {
            Some(host) => {
                format!("https://{host}{path}")
            }
            None => path.to_string(),
        };

        println!("Sending to {:?} {}", method, url);

        let mut req = match method {
            "GET" => client.get(url.clone()),
            "POST" => client.post(url.clone()),
            _ => return,
        };

        req = req.headers(headers);

        if method == "POST" {
            req = req.body(body.to_string());
        }

        let res = req.send().await;

        if let Ok(res) = res {
            let mut response = Res::default();
            response.url = res.url().to_string();
            response.status = res.status().to_string();
            response.headers = res
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let body_bytes = res.bytes().await.unwrap();
            let body_string = String::from_utf8_lossy(&body_bytes).to_string();

            response.body = body_string;

            let _ = app.emit("forwarded-response-received", json!(response));
        }
    }
}

#[tauri::command]
fn toggle_intercept(state: State<'_, Arc<AppState>>, intercept_toggle: bool) {
    state.intercept.store(intercept_toggle, Ordering::Relaxed);
    println!("Intercept toggled: {}", intercept_toggle);
}

#[derive(Default, Serialize, Deserialize)]
struct JwtNote {
    importance: String, // error, warning, info
    note: String,
}

impl JwtNote {
    pub fn new(importance: String, note: String) -> Self {
        JwtNote { importance, note }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct JwtParseResult {
    header: Option<String>,
    payload: Option<String>,
    signature: Option<String>,
    notes: Vec<JwtNote>,
}

const REQUIRED_CLAIMS: [&'static str; 3] = ["exp", "nbf", "aud"];

#[tauri::command]
fn parse_jwt_token(raw_token: String, secret: String) -> JwtParseResult {
    let parts = raw_token.split(".").collect::<Vec<&str>>();
    let mut results = JwtParseResult::default();
    if parts.len() != 3 {
        results.notes.push(JwtNote::new(
            "error".to_string(),
            "Invalid JWT Token: must have 3 parts".to_string(),
        ));
    }

    let header = URL_SAFE_NO_PAD
        .decode(parts[0].as_bytes())
        .unwrap_or_else(|_| {
            results.notes.push(JwtNote::new(
                "error".to_string(),
                "Invalid token: Invalid base64 header".to_string(),
            ));
            "{}".as_bytes().to_vec()
        });

    let payload = URL_SAFE_NO_PAD
        .decode(parts[1].as_bytes())
        .unwrap_or_else(|_| {
            results.notes.push(JwtNote::new(
                "error".to_string(),
                "Invalid token: Invalid base64 payload".to_string(),
            ));
            "{}".as_bytes().to_vec()
        });

    let header = String::from_utf8_lossy(&header[..]).to_string();
    let payload = String::from_utf8_lossy(&payload[..]).to_string();

    let header_json = serde_json::from_str(&header).unwrap_or_else(|_| {
        results.notes.push(JwtNote::new(
            "warning".to_string(),
            "Invalid header JSON".to_string(),
        ));
        Value::default()
    });

    let payload_json = serde_json::from_str(&payload).unwrap_or_else(|_| {
        results.notes.push(JwtNote::new(
            "warning".to_string(),
            "Invalid payload JSON".to_string(),
        ));
        Value::default()
    });

    results.header = Some(header);
    results.payload = Some(payload);
    results.signature = Some(parts[2].to_string());
    println!("{}", payload_json);

    for claim in REQUIRED_CLAIMS {
        if payload_json.get(claim).is_none() {
            results.notes.push(JwtNote::new(
                "warning".to_string(),
                format!("Claim {} not found in JWT", claim).to_string(),
            ));
        }
    }

    if let Some(alg) = header_json.get("alg") {
        if let Some(alg) = alg.as_str() {
            let algorithm: Algorithm = alg.parse().unwrap_or_else(|_| {
                results.notes.push(JwtNote::new(
                    "error".to_string(),
                    "Invalid algorithm".to_string(),
                ));
                Algorithm::default()
            });

            let mut validation = Validation::new(algorithm);
            validation.validate_aud = false;
            validation.validate_exp = false;
            validation.validate_nbf = false;
            validation.required_spec_claims.clear();

            match decode::<serde_json::Value>(
                &raw_token,
                &DecodingKey::from_secret(secret.as_bytes()),
                &validation,
            ) {
                Ok(_token_data) => {
                    results.notes.push(JwtNote::new(
                        "info".to_string(),
                        "Valid signature".to_string(),
                    ));
                }
                Err(err) => {
                    println!("{err}");
                    results.notes.push(JwtNote::new(
                        "error".to_string(),
                        "Invalid signature".to_string(),
                    ));
                }
            }
        } else {
            results.notes.push(JwtNote::new(
                "error".to_string(),
                "Algorithm claim is not a string".to_string(),
            ));
        }
    } else {
        results.notes.push(JwtNote::new(
            "error".to_string(),
            "Token does not have an algorithm".to_string(),
        ));
    }

    return results;
}

#[derive(Default, Serialize, Deserialize)]
struct JwtEncodeResult {
    header: String,
    payload: String,
    signature: String,
    notes: Vec<JwtNote>,
}

#[tauri::command]
fn encode_jwt(header: String, payload: String, secret: String) -> JwtEncodeResult {
    let mut result = JwtEncodeResult::default();
    let _ = match serde_json::from_str::<Value>(&payload) {
        Ok(vals) => vals,
        Err(e) => {
            result.notes.push(JwtNote::new(
                "error".to_string(),
                "Invalid payload JSON".to_string(),
            ));
            return result;
        }
    };

    let header_json = match serde_json::from_str::<Value>(&header) {
        Ok(vals) => vals,
        Err(e) => {
            result.notes.push(JwtNote::new(
                "error".to_string(),
                "Invalid payload JSON".to_string(),
            ));
            return result;
        }
    };

    let header_encoded = URL_SAFE_NO_PAD.encode(header);
    let payload_encoded = URL_SAFE_NO_PAD.encode(payload);

    if let Some(alg) = header_json.get("alg") {
        if let Ok(algorithm) = alg.as_str().unwrap().parse::<Algorithm>() {
            let message = format!("{}.{}", header_encoded, payload_encoded);
            let key = EncodingKey::from_secret(secret.as_bytes());
            result.signature = match sign(&message.as_bytes(), &key, algorithm) {
                Ok(val) => val,
                Err(_) => {
                    result.notes.push(JwtNote::new(
                        "error".to_string(),
                        "Failed to create signature".to_string(),
                    ));
                    return result;
                }
            };
        }
    }

    result.header = header_encoded;
    result.payload = payload_encoded;

    return result;
}

#[derive(Serialize, Deserialize, Clone)]
struct Dir {
    url: String,
    status: String,
}


#[tauri::command]
async fn probe_dirs(host: String, wordlist: String, is_file: bool, state: AppHandle) {
    let accepted_codes = Arc::new(vec![StatusCode::OK, StatusCode::CREATED, StatusCode::ACCEPTED, StatusCode::MOVED_PERMANENTLY, StatusCode::FORBIDDEN]);

    let file = File::open(wordlist).await.unwrap();
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let client = Client::new();
    let semaphore = Arc::new(Semaphore::new(200));
    let mut handles = vec![];
    let mut host = host;
    if host.ends_with("/") {
        host.pop();
    }

    println!("Starting directory probing");

    loop {
        let dir = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(_) => continue,
        };

        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let state = state.clone();
        let client = client.clone();
        let host = host.clone();
        let dir = dir.to_string();
        let accepted_clone = accepted_codes.clone();

        let handle = tokio::spawn(async move {
            let url = format!("{}/{}", host, dir);

            if let Ok(resp) = client.get(&url).send().await {
                if !accepted_clone.contains(&resp.status()) {
                    return;
                }

                let _ = state.emit(
                    "dir-received",
                    Dir {
                        url,
                        status: resp.status().as_str().to_string(),
                    },
                );
            }

            drop(permit);
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }
    let _ = state.emit("dir-scanning-finished", {});
}

fn main() {
    run();
}
