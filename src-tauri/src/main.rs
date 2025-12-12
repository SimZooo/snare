// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::crypto::sign;
use jsonwebtoken::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snare_script::Script;
use tokio::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::{collections::HashMap, sync::Arc};
use tauri::Manager;
use log::error;

mod network;
mod proxy;
mod script;

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
    headers: Vec<(String, String)>,
    body: String,
    raw: String,
}

struct AppState {
    intercept: AtomicBool,
    scripts: Arc<Mutex<HashMap<String, (Script, String, bool)>>>
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState {
        intercept: AtomicBool::new(false),
        scripts: Arc::new(Mutex::new(HashMap::new()))
    });

    let state_clone = state.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let task = tauri::async_runtime::spawn(async move {
                proxy::start_proxy(app_handle, state_clone).await.unwrap();
            });

            app.manage(task);

            Ok(())
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            network::toggle_intercept,
            network::send_request,
            parse_jwt_token,
            encode_jwt,
            network::probe_dirs,
            network::bruteforce,
            script::run_script,
            script::get_args,
            script::update_script,
            script::remove_script,
            script::add_script
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
                    error!("{err}");
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
        Err(_) => {
            result.notes.push(JwtNote::new(
                "error".to_string(),
                "Invalid payload JSON".to_string(),
            ));
            return result;
        }
    };

    let header_json = match serde_json::from_str::<Value>(&header) {
        Ok(vals) => vals,
        Err(_) => {
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

fn main() {
    env_logger::init();
    run();
}
