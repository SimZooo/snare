// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{collections::HashMap, sync::Arc};

use serde::Serialize;
use tauri::Emitter;

struct AppState {
    app_handle: tauri::AppHandle,
}

#[derive(Clone, Serialize)]
struct AppRequest {
    method: String,
    uri: String,
    headers: HashMap<String, String>,
    body: String,
    raw: String,
}

#[derive(Clone, Serialize)]
struct AppResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    raw: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = Arc::new(AppState {
                app_handle: app.handle().clone(),
            });
            tauri::async_runtime::spawn(run_proxy(state));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use hyper::{Client, Server, Request, Response, Body, StatusCode, Method};
use hyper::service::{make_service_fn, service_fn};
use std::{convert::Infallible, net::SocketAddr};

async fn run_proxy(state: Arc<AppState>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    
    // Clone the Arc for the service factory
    let state_clone = Arc::clone(&state);
    
    let make_svc = make_service_fn(move |_conn| {
        // Clone the Arc for each service instance
        let state_service = Arc::clone(&state_clone);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, Arc::clone(&state_service))
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    
    println!("HTTP/HTTPS Proxy server running on http://localhost:3000");
    
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

async fn handle_request(
    req: Request<Body>, 
    state: Arc<AppState>
) -> Result<Response<Body>, Infallible> {
    // Store the original method and body before consuming the request
    let original_method = req.method().clone();
    let original_uri = req.uri().to_string();
    
    // Convert headers to HashMap before consuming the request
    let mut headers = HashMap::new();
    for (key, value) in req.headers() {
        headers.insert(
            key.as_str().to_string(),
            value.to_str().unwrap_or("").to_string()
        );
    }
    
    // Extract body
    let (parts, body) = req.into_parts();
    let body_bytes = hyper::body::to_bytes(body).await.unwrap_or_default();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    
    // Build raw HTTP packet for the request and create the AppRequest to emit
    let request_target = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or(original_uri.clone());

    let mut raw_request = format!("{} {} HTTP/1.1\r\n", original_method, request_target);
    for (k, v) in &headers {
        raw_request.push_str(&format!("{}: {}\r\n", k, v));
    }
    raw_request.push_str("\r\n");
    raw_request.push_str(&body_str);

    let app_request = AppRequest {
        method: original_method.to_string(),
        uri: original_uri.clone(),
        headers,
        body: body_str.clone(),
        raw: raw_request,
    };

    if let Err(e) = state.app_handle.emit("request-intercepted", &app_request) {
        eprintln!("Failed to emit request event: {}", e);
    } else {
        println!("Emitted event")
    }
    
    // Handle CONNECT method for HTTPS
    if original_method == Method::CONNECT {
        return Ok(Response::new(Body::from("HTTP proxy does not support HTTPS tunneling")));
    }
    
    // Rebuild the request for forwarding using the original method
    let client = Client::new();
    let mut new_req = Request::builder()
        .method(original_method) // Use the original Method, not String
        .uri(&app_request.uri);
    
    // Copy headers (excluding host)
    for (key, value) in &app_request.headers {
        if key.to_lowercase() != "host" {
            new_req = new_req.header(key, value);
        }
    }
    
    let new_req = new_req.body(Body::from(body_bytes)).unwrap();
    
    match client.request(new_req).await {
        Ok(response) => {
            let status = response.status();
            let response_headers = response.headers().clone();
            let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap_or_default();
            let body_str = String::from_utf8_lossy(&body_bytes).to_string();
            
            // Convert response headers to HashMap
            let mut headers_map = HashMap::new();
            for (key, value) in &response_headers {
                headers_map.insert(
                    key.as_str().to_string(),
                    value.to_str().unwrap_or("").to_string()
                );
            }

            let (tx, rx) = tokio::sync::oneshot::channel::<String>();
            let _ = rx.await;
            // Create response object for frontend
            // Build raw HTTP packet for the response
            let reason = status.canonical_reason().unwrap_or("");
            let mut raw_response = format!("HTTP/1.1 {} {}\r\n", status.as_u16(), reason);
            for (key, value) in &response_headers {
                raw_response.push_str(&format!("{}: {}\r\n", key.as_str(), value.to_str().unwrap_or("")));
            }
            raw_response.push_str("\r\n");
            raw_response.push_str(&body_str);

            let app_response = AppResponse {
                status: status.as_u16(),
                headers: headers_map,
                body: body_str.clone(),
                raw: raw_response,
            };
            
            // Emit event to frontend with the intercepted response
            if let Err(e) = state.app_handle.emit("response-intercepted", &app_response) {
                eprintln!("Failed to emit response event: {}", e);
            }
            
            println!("=== INTERCEPTED RESPONSE ===");
            println!("Status: {}", status);
            
            // Rebuild response for the client
            let mut response_builder = Response::builder().status(status);
            for (key, value) in &response_headers {
                response_builder = response_builder.header(key, value);
            }
            
            Ok(response_builder.body(Body::from(body_bytes)).unwrap())
        }
        Err(e) => {
            eprintln!("Proxy error: {}", e);
            
            // Emit error event to frontend
            let err_body = format!("Proxy error: {}", e);
            let mut raw_err = format!("HTTP/1.1 500 Internal Server Error\r\n");
            raw_err.push_str("\r\n");
            raw_err.push_str(&err_body);

            let error_response = AppResponse {
                status: 500,
                headers: HashMap::new(),
                body: err_body.clone(),
                raw: raw_err,
            };

            if let Err(emit_err) = state.app_handle.emit("response-intercepted", &error_response) {
                eprintln!("Failed to emit error event: {}", emit_err);
            }

            let (tx, rx) = tokio::sync::oneshot::channel::<String>();
            let _ = rx.await;
            
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Proxy error"))
                .unwrap())
        }
    }
}

fn main() {
    run();
}