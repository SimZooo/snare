use std::{collections::HashMap, error::Error, io, ops::Deref, process::exit, sync::{Arc, atomic::Ordering}, time::Duration};

use hyper::HeaderMap;
use log::{error, info};
use rcgen::{Issuer, KeyPair};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snare_script::Script;
use tauri::{AppHandle, Emitter, State, http::{HeaderName, HeaderValue}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::Mutex, time::sleep};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use uuid::Uuid;

use crate::{AppState, network::{create_server_config, generate_cert, get_domain, load_ca, read_request}};

fn parse_request(raw: String, id: String) -> io::Result<FlowRequest> {
    let mut lines = raw.split("\r\n");
    let Some(status_line) = lines.next() else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, 
            format!("Malformed request")))
    };

    let mut status_line_split = status_line.split_whitespace();
    let Some(method) = status_line_split.next() else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, 
            format!("Malformed request when parsing method")))
    };

    let Some(path) = status_line_split.next() else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, 
            format!("Malformed request when parsing path")))
    };

    let mut raw_split = raw.split("\r\n\r\n");
    let Some(headers) = raw_split.next() else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, 
            format!("Malformed request when parsing headers")))
    };

    let body = raw_split.next().unwrap_or("");

    let host = headers.split("\r\n").find(|line| line.to_lowercase().starts_with("host")).unwrap_or("").split(":").skip(1).next().unwrap_or("");
    let req = FlowRequest::new(id.to_string(), method.to_string(), path.to_string(), host.to_string(), headers.to_string(), body.to_string(), raw.to_string());

    Ok(req)
}

async fn parse_response(res: Response, id: String) -> io::Result<FlowResponse> {
    let status = res.status().to_string();
    let headers = res.headers()
    .iter()
    .map(|(k, v)| {
        (
            k.to_string(),
            v.to_str().unwrap_or("<invalid utf8>").to_string(),
        )
    })
    .collect::<Vec<(String, String)>>();
    let body = res.text().await.map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, 
            format!("Failed parsing body: {e}"))
    })?;

    let status_line = format!("HTTP/1.1 {status}\r\n");

    let mut headers_raw = headers.iter().map(|(k, v)| {
        if !v.starts_with("transfer-encoding") {
            let v = v.to_string();
            format!("{k}: {v}\r\n")
        } else {
            String::new()
        }
    }).collect::<Vec<String>>();
    if !headers.iter().any(|(k, _)| k.to_lowercase() == "content-length") {
        headers_raw.push(format!("content-length: {}", body.bytes().len().to_string()));
    }

    let raw = vec![status_line, headers_raw.join(""), "\r\n".to_string(), body.clone()].join("");

    Ok(FlowResponse::new(id.to_string(), status.to_string(), headers, body.to_string(), raw))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlowRequest {
    id: String,
    method: String,
    path: String,
    host: String,
    headers: String,
    body: String,
    raw: String
}

impl FlowRequest {
    fn new(id: String, method: String, path: String, host: String, headers: String, body: String, raw: String) -> Self {
        FlowRequest { id, method, path, host, headers, body, raw }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlowResponse {
    id: String,
    status: String,
    headers: Vec<(String, String)>,
    body: String,
    raw: String,
}

impl FlowResponse {
    fn new(id: String, status: String, headers: Vec<(String, String)>, body: String, raw: String) -> Self {
        FlowResponse { id, status, headers, body, raw }
    }
}

#[derive(Debug)]
enum Flow {
    Request(FlowRequest),
    Response(FlowResponse),
}

async fn handle_server_connection(tx: Arc<tokio::sync::mpsc::Sender<Flow>>, tls_stream: &mut TlsStream<TcpStream>, req_raw: String, scripts: &Arc<Mutex<HashMap<String, (Script, String, bool)>>>) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let scripts = scripts.lock().await;
    let mut req = req_raw.clone();
    info!("Scripts: {:?}", scripts.keys());

    // Iterate through each script and chain
    for (script, args, enabled) in scripts.values() {
        info!("Running script: {}", script.metadata.name);
        if !enabled {
            continue;
        }
        req = script.execute(req.clone(), args.clone()).await.map_err(|e| {
            error!("{e}");
            io::Error::new(io::ErrorKind::Other, format!("ScriptError: {e}"))
        })?;
        info!("Script result: {}", req);
    }
    // Receive from client
    let id = Uuid::new_v4().to_string();
    let _ = tx.send(Flow::Request(parse_request(req.clone(), id.clone())?)).await;
    info!("Flow sent to receiver");

    // Send to and receive from server
    info!("Forwarding to client");
    let res = forward_to_server(req).await?;
    info!("Parsing response");
    let flow_res = Flow::Response(parse_response(res, id).await?);
    // Send response back to client
    if let Flow::Response(res) = &flow_res {
        let _ = tls_stream.write_all(res.raw.as_bytes()).await;
        let _ = tls_stream.flush().await;
    }

    let _ = tx.send(flow_res).await;
    info!("Sent response flow");
    Ok(())
}

pub async fn start_proxy(app_handle: AppHandle, state: Arc<AppState>) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    info!("Started proxy");
    let listener = match TcpListener::bind("127.0.0.1:3009").await {
        Ok(listener) => listener,
        Err(e) => { error!("Failed to bind listener {e}"); exit(1)}
    };

    let issuer = Arc::new(load_ca().await.unwrap());
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Flow>(100);
    let tx = Arc::new(tx);
    let state_clone = state.clone();
    let scripts = state.scripts.clone();

    tokio::spawn(async move {
        loop {
            let scripts_clone = scripts.clone();
            if !state_clone.intercept.load(Ordering::Relaxed) {
                continue;
            }

            if let Ok((stream, _)) = listener.accept().await {
                let issuer = issuer.clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    let mut tls_stream = handle_client_connection(stream, issuer).await?;
                    loop {
                        let req_raw = match read_http_request(&mut tls_stream).await {
                            Ok(r) => r,
                            Err(_) => break
                        };
                        let _ = handle_server_connection(tx.clone(), &mut tls_stream, req_raw, &scripts_clone).await?;
                    }

                    Ok::<(), Box<dyn Error + Send + Sync + 'static>>(())
                });
            }
            sleep(Duration::from_millis(100)).await;
        }
    });

    while let Some(flow) = rx.recv().await {
        if let Flow::Request(req) = &flow {
            let _ = app_handle.emit("request-received", json!(req)).inspect_err(|e| error!("Flow receiver error (request): {e}"));
        } else if let Flow::Response(res) = &flow {
            let _ = app_handle.emit("response-received", json!(res)).inspect_err(|e| error!("Flow receiver error (response): {e}"));
        }
    }

    info!("Stopped proxy");
    Ok(())
}

async fn handle_client_connection(mut stream: TcpStream, issuer: Arc<Issuer<'static, KeyPair>>) -> io::Result<TlsStream<TcpStream>> {
    let req = read_request(&mut stream).await?;
    if !req.starts_with("CONNECT") {
        return Err(io::Error::new(io::ErrorKind::Other, "Expected CONNECT"));
    }

    // Generate cert and key-pair for domain
    let domain_line = get_domain(&req)?;
    let domain_line = domain_line.split(":").collect::<Vec<&str>>();
    let domain = domain_line[0];
    let (cert, key) = generate_cert(domain.to_string(), issuer).await?;
    stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;

    let cert_der = cert.der();
    let key_der = key.serialize_der();
    let server_config = create_server_config(cert_der.to_vec(), key_der).await?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    let mut tls_stream = tls_acceptor.accept(stream).await?;

    Ok(tls_stream)
}

async fn read_http_request(tls_stream: &mut TlsStream<TcpStream>) -> io::Result<String> {
    loop {
        let mut buf = vec![0u8; 4096];
        let n = tls_stream.read(&mut buf[..]).await?;
        if n == 0 {
            break;
        }
        let raw = String::from_utf8_lossy(&buf[..n]);
        return Ok(raw.to_string())
    }

    Ok("".to_string())
}

async fn forward_to_server(raw: String) -> io::Result<Response> {
    let client = Client::new();
    let raw_owned = raw.clone();

    if let Some((headers_str, body)) = raw_owned.split_once("\r\n\r\n") {
        let mut headers = HeaderMap::new();

        let headers_split: Vec<String> = headers_str
            .lines()
            .skip(1)
            .map(|line| line.to_string())
            .collect();

        let Some(request_line) = headers_str.lines().next() else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Malformed request data"))
        };
        let (method, path, _version) =
            match request_line.split_whitespace().collect::<Vec<&str>>()[..] {
                [m, p, v] => (m, p, v),
                _ => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, 
                        format!("Malformed request line")))
                }
            };

        let mut host = None;
        for header in headers_split {
            if let Some((key, value)) = header.split_once(":") {
                if key.to_lowercase() == "host" {
                    host = Some(value.trim().to_string());
                }

                let hname = HeaderName::from_bytes(key.trim().as_bytes()).map_err(|e|
                    io::Error::new(io::ErrorKind::InvalidData, format!("Invalid header data: {e}"))
                )?;
                let hvalue = HeaderValue::from_str(value.trim()).map_err(|e|
                    io::Error::new(io::ErrorKind::InvalidData, format!("Invalid header data: {e}"))
                )?;
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

        let mut req = match method {
            "GET" => client.get(url.clone()),
            "POST" => client.post(url.clone()),
            _ => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, 
                    format!("Invalid request method")))
            },
        };

        req = req.headers(headers);

        if method == "POST" {
            req = req.body(body.to_string());
        }

        let res = req.send().await.map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, 
                format!("Failed sending request to server: {}", e))
        })?;

        return Ok(res)
    }

    Err(io::Error::new(io::ErrorKind::InvalidData, 
        format!("Malformed request in forward_to_client")))
}
