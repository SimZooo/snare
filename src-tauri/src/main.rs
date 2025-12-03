// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use base64::prelude::BASE64_STANDARD;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use jsonwebtoken::crypto::sign;
use jsonwebtoken::*;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{collections::HashMap, process::Stdio, sync::Arc};
use tauri::http::{HeaderMap, HeaderName, HeaderValue};
use tauri::{AppHandle, Emitter, State};
use tokio::fs::File;
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
        .plugin(tauri_plugin_fs::init())
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
            probe_dirs,
            bruteforce
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
        if header_json.get(claim).is_none() {
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
async fn probe_dirs(host: String, wordlist: String, rate_limit: usize, state: AppHandle) {
    let accepted_codes = Arc::new(vec![
        StatusCode::OK,
        StatusCode::CREATED,
        StatusCode::ACCEPTED,
        StatusCode::MOVED_PERMANENTLY,
        StatusCode::FORBIDDEN,
    ]);

    let file = File::open(wordlist).await.unwrap();
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut interval = tokio::time::interval(Duration::from_millis(
        (1000.0 / rate_limit as f64) as u64
    ));
    let client = Client::new();
    let semaphore = Arc::new(Semaphore::new(200));
    let mut handles = vec![];
    let mut host = host;

    if host.ends_with("/") {
        host.pop();
    }

    println!("Starting directory probing");

    loop {
        if rate_limit != 0 {
            interval.tick().await;
        }
        let dir = match lines.next_line().await {
            Ok(Some(line)) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                trimmed
            },
            Ok(None) => break,
            Err(_) => continue,
        };

        let state = state.clone();
        let client = client.clone();
        let host = host.clone();
        let dir = dir.to_string();
        let accepted_clone = accepted_codes.clone();
        let semaphore = semaphore.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.unwrap();
            let url = format!("{}/{}", host, dir);

            if let Ok(resp) = client.get(&url).send().await {
                if accepted_clone.contains(&resp.status()) {
                    let _ = state.emit(
                        "dir-received",
                        Dir {
                            url,
                            status: resp.status().as_str().to_string(),
                        },
                    );
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }
    let _ = state.emit("dir-scanning-finished", {});
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttackType {
    Form,
    Basic
}

#[tauri::command]
async fn bruteforce(file_paths: Vec<String>, attack_type: String, url: String, app_handle: AppHandle) {
    let attack_type = match attack_type.as_str() {
        "form" => AttackType::Form,
        "basic" => AttackType::Basic,
        _ => {
            AttackType::Basic
        }
    };

    let read_lines = |path: &PathBuf| -> io::Result<Vec<String>> {
        let file = std::fs::File::open(path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let reader = std::io::BufReader::new(file);
        Ok(reader.lines().filter_map(Result::ok).collect())
    };

    let client = Client::new();

    let users_path = PathBuf::from(Path::new(&file_paths[0]));
    let users = match read_lines(&users_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to read users file: {e}");
            Vec::new()
        }
    };

    let users_path = PathBuf::from(Path::new(&file_paths[1]));
    let passwords = match read_lines(&users_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to read users file: {e}");
            Vec::new()
        }
    };

    let responses: Vec<(String, String, String)> = send_reqs(Arc::new(client), url.clone(), users, passwords, attack_type).await.iter().map(|val| (val.0.clone(), val.1.clone(), url.clone())).collect();
    
    app_handle.emit("bruteforce-responses", responses);
}

async fn send_req(client: Arc<Client>, url: &String, user: Arc<String>, pass: String, method: Method, attack_type: AttackType) -> anyhow::Result<Option<(String, String)>> {
    let mut request;
    match method {
        Method::GET => {
            request = client.get(url);
        }, 
        Method::POST => {
            request = client.post(url);
        },
        _ => {
            return Err(anyhow::anyhow!("Invalid method given"))
        }
    };

    match attack_type {
        AttackType::Form => {
            let body = format!("username={}&password={}", user, pass);
            request = request
                .body(body)
                .header("Content-Type", "application/x-www-form-urlencoded");
        },
        AttackType::Basic => {
            let auth_header = BASE64_STANDARD.encode(format!("{}:{}", user, pass));
            request = request.header("Authorization", format!("Basic {}", auth_header));
        }
    }

    let res = request.send().await?;
    let status = res.status();
    
    if status.is_success() || status.is_redirection() {
        return Ok(Some((user.to_string(), pass)))
    }

    Ok(None)
}

async fn send_reqs(
    client: Arc<Client>,
    url: String,
    users: Vec<String>,
    passwords: Vec<String>,
    attack_type: AttackType
) -> Vec<(String, String)> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(String, String)>(4096);
    let url = Arc::new(url);
    let method = Arc::new(match attack_type {
        AttackType::Basic => Method::GET,
        AttackType::Form => Method::POST,
    });

    let mut futures = FuturesUnordered::new();

    for user in users {
        let user = Arc::new(user);
        for pass in passwords.iter() {
            let client = client.clone();
            let user = user.clone();
            let url = url.clone();
            let method = method.clone();
            let tx = tx.clone();
            let pass = pass.clone();

            futures.push(async move {
                let res = 
                send_req(
                    client, 
                    &url, 
                    user, 
                    pass, 
                    (*method).clone(), 
                    attack_type
                ).await;

                if let Ok(Some(found)) = res {
                    let _ = tx.send(found).await;
                }
            });
        }
    }

    drop(tx);

    while futures.next().await.is_some() {}

    let mut results = Vec::new();
    while let Some(r) = rx.recv().await {
        results.push(r);
    }

    results
}

fn main() {
    run();
}
