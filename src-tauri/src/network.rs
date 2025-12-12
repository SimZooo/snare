use std::{fs::read_to_string, io::{self, BufRead}, path::{Path, PathBuf}, sync::{Arc, atomic::Ordering}, time::Duration};
use base64::{Engine, prelude::BASE64_STANDARD};
use futures::{StreamExt, stream::FuturesUnordered};
use hyper::{HeaderMap, Method, StatusCode};
use rcgen::{Certificate, CertificateParams, DnType, Issuer, KeyPair};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State, http::{HeaderName, HeaderValue}};
use tokio::{fs::File, io::{AsyncBufReadExt, AsyncReadExt, BufReader}, net::TcpStream, sync::Semaphore};
use tokio_rustls::rustls::{ServerConfig, pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer}};
use serde_json::json;
use log::{info, error};

use crate::{AppState, Res};

pub async fn read_request(stream: &mut TcpStream) -> io::Result<String> {
    let mut buffer = [0u8; 4096];
    let n = match stream.read(&mut buffer[..]).await {
        Ok(n) => n,
        Err(e) => { error!("Failed to read data from stream: {e}. Error in check_connect"); return Err(e)}
    };
    let request = String::from_utf8_lossy(&buffer[..n]);
    Ok(request.to_string())
}

pub async fn load_ca() -> io::Result<Issuer<'static, KeyPair>> {
    let ca_key_path = "resources/private/ca.key.unencrypted";
    let ca_cert_path = "resources/certs/ca.crt";

    info!("Loading CA key from {}", ca_key_path);
    info!("Loading CA cert from {}", ca_cert_path);

    let ca_key_pem = read_to_string(ca_key_path).map_err(|e| {
        error!("ERROR: Could not read CA certificate file at '{}': {}", ca_cert_path, e);
        io::Error::new(io::ErrorKind::NotFound, 
            format!("Failed to read CA certificate from {}: {}", ca_cert_path, e))
    })?;

    let ca_cert_pem = read_to_string(ca_cert_path).map_err(|e| {
        error!("ERROR: Could not read CA key file at '{}': {}", ca_cert_path, e);
        io::Error::new(io::ErrorKind::NotFound, 
            format!("Failed to read CA key from {}: {}", ca_cert_path, e))
    })?;

    let key_pair = KeyPair::from_pem(&ca_key_pem).map_err(|e| {
        error!("ERROR: Failed to parse CA key PEM: {}", e);
        io::Error::new(io::ErrorKind::InvalidData, 
            format!("Failed to parse CA key: {}", e))
    })?;

    let issuer = Issuer::from_ca_cert_pem(&ca_cert_pem, key_pair).map_err(|e| {
        error!("ERROR: Failed to parse CA certificate: {}", e);
        error!("Certificate content (first 200 chars): {}", 
            &ca_cert_pem.chars().take(200).collect::<String>());
        io::Error::new(io::ErrorKind::InvalidData, 
            format!("Failed to parse CA certificate: {}", e))
    })?;

    info!("Successfully loaded CA certificate");
    Ok(issuer)
}

pub async fn generate_cert(domain: String, issuer: Arc<Issuer<'static, KeyPair>>) -> io::Result<(Certificate, KeyPair)> {
    // Create cert
    let mut params = CertificateParams::new(vec![domain.to_string()]).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, 
            format!("Failed to create certificate params: {}", e))
    })?;
    params.distinguished_name.push(DnType::CommonName, domain);

    let key_pair = KeyPair::generate().unwrap();

    let cert = params.signed_by(&key_pair, &issuer).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, 
            format!("Failed to sign certificate params: {}", e))
    })?;

    Ok((cert, key_pair))
}

pub fn get_domain(request: &String) -> io::Result<String>{
    let lines = request.split("\r\n").collect::<Vec<&str>>();
    let status_line = match lines.first() {
        Some(f) => f,
        None => { return Err(io::Error::new(io::ErrorKind::Other, "Empty request")) }
    };
    let mut split_status_line = status_line.split_whitespace();
    let _ = split_status_line.next();
    let domain = match split_status_line.next() {
        Some(f) => f,
        None => { return Err(io::Error::new(io::ErrorKind::Other, "No target specified in status line")) }
    };
    Ok(domain.to_string())
}

pub async fn create_server_config(cert_der: Vec<u8>, key_der: Vec<u8>) -> io::Result<ServerConfig>{
    let cert = CertificateDer::from(cert_der);
    //let key = PrivateKeyDer::from_pem(SectionKind::RsaPrivateKey, key_der).unwrap();
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_der));

    let config = ServerConfig::builder().with_no_client_auth().with_single_cert(vec![cert], key).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, 
            format!("Failed to build serverconfig: {}", e))
    })?;

    Ok(config)
}

#[tauri::command]
pub async fn send_request(app: AppHandle, raw: String) {
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
                    error!("Invalid request line: {}", request_line);
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

        info!("Sending to {:?} {}", method, url);

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
            response.body = body_string.clone();
            response.raw = [format!("HTTP/1.1 {}", response.status), response.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<String>>().join("\r\n"), "".to_string(), body_string].join("\r\n");

            let _ = app.emit("forwarded-response-received", json!(response));
        }
    }
}

#[tauri::command]
pub fn toggle_intercept(state: State<'_, Arc<AppState>>, intercept_toggle: bool) {
    state.intercept.store(intercept_toggle, Ordering::Relaxed);
    info!("Intercept toggled: {}", intercept_toggle);
}

pub async fn send_req(client: Arc<Client>, url: &String, user: Arc<String>, pass: String, method: Method, attack_type: AttackType) -> anyhow::Result<Option<(String, String)>> {
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

#[derive(Serialize, Deserialize, Clone)]
struct Dir {
    url: String,
    status: String,
}

#[tauri::command]
pub async fn probe_dirs(host: String, wordlist: String, rate_limit: usize, state: AppHandle) {
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

    info!("Starting directory probing");

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
pub async fn bruteforce(file_paths: Vec<String>, attack_type: String, url: String, app_handle: AppHandle) {
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
            error!("Failed to read users file: {e}");
            Vec::new()
        }
    };

    let users_path = PathBuf::from(Path::new(&file_paths[1]));
    let passwords = match read_lines(&users_path) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to read users file: {e}");
            Vec::new()
        }
    };

    let responses: Vec<(String, String, String)> = send_reqs(Arc::new(client), url.clone(), users, passwords, attack_type).await.iter().map(|val| (val.0.clone(), val.1.clone(), url.clone())).collect();
    
    let _ = app_handle.emit("bruteforce-responses", responses);
}