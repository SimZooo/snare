use std::sync::Arc;

use log::{info, warn};
use serde_json::Value;
use snare_script::Script;
use tauri::State;
use serde_json::json;

use crate::AppState;

#[tauri::command]
pub async fn add_script(state: State<'_, Arc<AppState>>, path: String, args: Vec<Value>) -> Result<(), String> {
    info!("Adding script from path: {}", path);
    let script = Script::new(&path).map_err(|e| {
        format!("ScriptError: {e}").to_string()
    })?;
    
    info!("Script name from metadata: {:?}", script.metadata.name);
    
    let mut scripts = state.scripts.lock().await;
    let args_string = json!(args).to_string();
    scripts.insert(script.metadata.name.clone(), (script, args_string, true));
    
    info!("Current script keys: {:?}", scripts.keys().collect::<Vec<&String>>());
    
    Ok(())
}

#[tauri::command]
pub async fn remove_script(state: State<'_, Arc<AppState>>, name: String) -> Result<(), String> {
    let mut scripts = state.scripts.lock().await;
    if let Some(_) = scripts.remove(&name) {
        info!("Successfully removed script: {:?}", name);
    } else {
        warn!("Script {:?} not found!", name);
    }

    Ok(())
}

#[tauri::command]
pub async fn update_script(state: State<'_, Arc<AppState>>, name: String, args: Vec<Value>, enabled: bool) -> Result<(), String> {
    let mut scripts = state.scripts.lock().await;
    let args = json!(args).to_string();

    info!("Updating script with args: {} and state: {}", args, enabled);

    let Some((_, args_s, enabled_s)) = scripts.get_mut(&name) else {
        return Err("Script is not added. Run add_script first".to_string());
    };

    *args_s = args;
    *enabled_s = enabled;

    Ok(())
}

#[tauri::command]
pub async fn run_script(state: State<'_, Arc<AppState>>, name: String, request: String) -> Result<String, String> {
    let scripts = state.scripts.lock().await;

    let Some((script, args, enabled)) = scripts.get(&name) else {
        return Err("Script is not added. Run add_script first".to_string());
    };

    if *enabled {
        let res = script.execute(request, args.clone()).await.map_err(|e| {
            format!("ScriptError: {e}").to_string()
        })?;
        return Ok(res.to_string())
    }

    Ok("".to_string())
}

#[tauri::command]
pub async fn get_args(path: String) -> Option<Value> {
    let script = Script::new(&path).unwrap();
    let res = script.get_args().unwrap();
    let res_map = snare_script::get_table(&res)?;

    Some(res_map)
}
