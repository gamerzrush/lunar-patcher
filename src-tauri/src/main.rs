#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Utc, Duration};
use uuid::Uuid;

#[derive(Deserialize)]
struct MojangResponse { id: String }

#[derive(Serialize)]
struct AccountInfo {
    local_id: String, username: String, uuid: String, is_active: bool,
}

// --- HELPER 1: GENERATE TOKEN --- //
fn generate_token(uuid: &str, display_name: &str) -> (String, String) {
    let now = Utc::now();
    let expire_time = now + Duration::hours(48);
    let header = json!({"kid": "049181", "alg": "RS256"});
    let payload = json!({
        "xuid": "2923152968100075", "agg": "Adult", "sub": "a185d8d314214c5834061a453c145af2",
        "auth": "XBOX", "ns": "default", "roles": [], "iss": "authentication",
        "flags": ["orders_2022", "msamigration_stage4", "twofactorauth", "multiplexUr"],
        "profiles": {"mc": uuid}, "platform": "PC_LAUNCHER",
        "pfd": [{"type": "mc", "id": uuid, "name": display_name}],
        "nbf": now.timestamp(), "exp": expire_time.timestamp(), "iat": now.timestamp(),
        "aid": "00000000-0000-0000-0000-0000402b5328"
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
    let dummy_signature = "uIvE2780LDneZuyVWKGXCdTxnd79byezKuNFGx_mWOo-FQ5yHv5XwsNW66eElUnNOe6H6hk5pWWIu_5vs_9QIpUIhTiv_--anBg6i1zXesumKUQj82x7c1gcYZSJ10o1RwQ91-1Ftrk1FW4odlhFM0b1KpwaEAkrJcwxnrYYExeYgQ1ta1_ZZfe4w-gUWPcBivckXEDIINr0wbCJX8qszZFqWzjL8sGcDytcpAp-zEWHNLQPuAkpZAlJf7L8PWAF7JeYAoOFzWFtTBSFSjfmvldhAsk6nnmGIeuWbBbBoJFzZuA-ZA3YQfhWKHEdXtCJWYVsHkkg5Oi6-nmDERoGHw";
    
    (format!("{}.{}.{}", header_b64, payload_b64, dummy_signature), expire_time.format("%Y-%m-%dT%H:%M:%S.000Z").to_string())
}

// --- HELPER 2: ADD HOSTS PATCH (Waits for UAC) --- //
fn patch_hosts_file() -> Result<(), String> {
    let hosts_path = "C:\\Windows\\System32\\drivers\\etc\\hosts";
    if let Ok(content) = fs::read_to_string(hosts_path) {
        if content.contains("api.lunarclient.com") { return Ok(()); } // Already patched
    }

    let domains = vec![
        "authenticator.lunarclientprod.com", "launcherupdates.lunarclientcdn.com", "hwid.lunarclient.com", 
        "api.lunarclient.com", "analytics.lunarclient.com", "analytics.lunarclientprod.com", 
        "genesis-production.lunarclientprod.com", "production.lunarclientprod.com", "connect.lunarclientprod.com",
        "assetserver.lunarclient.com", "blog.lunarclient.com", "thirdpartycache.lunarclient.com",
        "t.lunarclientcdn.com", "console.lunarclient.com", "store.lunarclient.com", "support.lunarclient.com", 
        "status.lunarclient.com", "cdn.lunarclient.com",
        // --- Added Microsoft / Xbox Domains ---
        "login.live.com", "user.auth.xboxlive.com", 
        "xsts.auth.xboxlive.com", "api.minecraftservices.com"
    ];
    
    let mut patch_block = String::from("\n# --- LUNAR PATCHER ---\n");
    for d in domains { patch_block.push_str(&format!("0.0.0.0 {}\n", d)); }
    patch_block.push_str("# ---------------------\n");

    let temp_file = std::env::temp_dir().join("lunar_patch.txt");
    let _ = fs::write(&temp_file, patch_block);

    // Notice we added -Wait and Error checking here!
    let script = format!(
        "try {{ Start-Process cmd -ArgumentList '/c type \"{}\" >> C:\\Windows\\System32\\drivers\\etc\\hosts & ipconfig /flushdns' -Verb RunAs -Wait -WindowStyle Hidden -ErrorAction Stop }} catch {{ exit 1 }}",
        temp_file.display()
    );

    let status = std::process::Command::new("powershell")
        .args(&["-NoProfile", "-Command", &script])
        .status()
        .map_err(|_| "Failed to request Administrator permissions.".to_string())?;

    if !status.success() { return Err("Administrator permission was denied.".to_string()); }
    Ok(())
}

// --- HELPER 3: TASK SCHEDULER --- //
fn install_task_scheduler() {
    if let Ok(exe_path) = std::env::current_exe() {
        let path_str = exe_path.to_str().unwrap_or_default();
        let _ = std::process::Command::new("schtasks")
            // We changed "/sc daily" to "/sc onlogon", and removed the "/st 12:00" time!
            .args(&["/create", "/tn", "LunarPatcherRefresh", "/tr", &format!("\"{}\" --refresh", path_str), "/sc", "onlogon", "/f"])
            .output();
    }
}

// --- HELPER 4: BACKGROUND REFRESH --- //
fn background_refresh() {
    let home_dir = match dirs::home_dir() { Some(dir) => dir, None => return };
    let accounts_file = home_dir.join(".lunarclient").join("settings").join("game").join("accounts.json");
    if let Ok(content) = fs::read_to_string(&accounts_file) {
        if let Ok(mut parsed) = serde_json::from_str::<Value>(&content) {
            if let Some(accounts) = parsed["accounts"].as_object_mut() {
                for (_, acc_data) in accounts.iter_mut() {
                    let uuid = acc_data["minecraftProfile"]["id"].as_str().unwrap_or("").to_string();
                    let name = acc_data["minecraftProfile"]["name"].as_str().unwrap_or("").to_string();
                    if !uuid.is_empty() && !name.is_empty() {
                        let (token, exp) = generate_token(&uuid, &name);
                        acc_data["accessToken"] = json!(token);
                        acc_data["accessTokenExpiresAt"] = json!(exp);
                    }
                }
            }
            if let Ok(pretty) = serde_json::to_string_pretty(&parsed) { let _ = fs::write(&accounts_file, pretty); }
        }
    }
}

// ==========================================
// TAURI COMMANDS
// ==========================================
#[tauri::command]
async fn add_new_account(display_name: String, skin_name: String) -> Result<String, String> {
    let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", skin_name);
    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    
    let uuid = if response.status().is_success() {
        let data: MojangResponse = response.json().await.map_err(|e| e.to_string())?;
        data.id
    } else { return Err("Account not found.".to_string()); };

    let (access_token, expire_iso) = generate_token(&uuid, &display_name);

    let home_dir = dirs::home_dir().ok_or("Could not find Home Directory")?;
    let lunar_dir = home_dir.join(".lunarclient").join("settings").join("game");
    let accounts_file = lunar_dir.join("accounts.json");

    let mut accounts_data = json!({"activeAccountLocalId": "", "accounts": {}});
    if accounts_file.exists() {
        if let Ok(content) = fs::read_to_string(&accounts_file) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&content) { accounts_data = parsed; }
        }
    }

    let local_id = Uuid::new_v4().to_string().replace("-", "");
    accounts_data["accounts"][&local_id] = json!({
        "accessToken": access_token, "accessTokenExpiresAt": expire_iso, "eligibleForMigration": false,
        "hasMultipleProfiles": false, "legacy": false, "persistent": true, "localId": local_id,
        "refreshToken": "M.C535_BL2.0.U.-Cr6ZtOm!dummy_refresh_token",
        "minecraftProfile": {"id": uuid, "name": display_name},
        "remoteId": "2923152968100075", "type": "Xbox", "username": display_name, "userProperties": []
    });
    accounts_data["activeAccountLocalId"] = json!(local_id);

    fs::create_dir_all(&lunar_dir).map_err(|e| e.to_string())?;
    fs::write(&accounts_file, serde_json::to_string_pretty(&accounts_data).unwrap()).map_err(|e| e.to_string())?;

    // --- APPLY PATCHES (Now checks for errors and aborts if you click No!) ---
    patch_hosts_file()?;
    install_task_scheduler();

    Ok(format!("Added: {}", display_name))
}

// --- NEW COMMAND: REMOVE PATCHES ---
#[tauri::command]
fn remove_patches() -> Result<String, String> {
    // 1. Delete Scheduled Task
    let _ = std::process::Command::new("schtasks")
        .args(&["/delete", "/tn", "LunarPatcherRefresh", "/f"])
        .output();

    // 2. Clean the Hosts File
    let hosts_path = "C:\\Windows\\System32\\drivers\\etc\\hosts";
    if let Ok(content) = fs::read_to_string(hosts_path) {
        let mut new_lines = Vec::new();
        let mut skip_line = false;
        
        for line in content.lines() {
            // Check if we hit our custom block
            if line.contains("# --- LUNAR PATCHER ---") { skip_line = true; continue; }
            if line.contains("# ---------------------") { skip_line = false; continue; }
            
            // Skip individual lines just in case they were added manually
            if skip_line || line.contains("lunarclient") || line.contains("xboxlive.com") || line.contains("api.minecraftservices.com") { continue; }
            
            new_lines.push(line.to_string());
        }

        let cleaned_content = new_lines.join("\n") + "\n";
        let temp_file = std::env::temp_dir().join("lunar_unpatch.txt");
        fs::write(&temp_file, cleaned_content).map_err(|e| e.to_string())?;

        // Ask for UAC to overwrite the hosts file with the cleaned version
        let script = format!(
            "try {{ Start-Process cmd -ArgumentList '/c copy /y \"{}\" C:\\Windows\\System32\\drivers\\etc\\hosts & ipconfig /flushdns' -Verb RunAs -Wait -WindowStyle Hidden -ErrorAction Stop }} catch {{ exit 1 }}",
            temp_file.display()
        );

        let status = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-Command", &script])
            .status()
            .map_err(|_| "Failed to request Administrator permissions.".to_string())?;

        if !status.success() { return Err("Administrator permission was denied.".to_string()); }
    }
    
    Ok("System Patches Removed!".to_string())
}

#[tauri::command]
fn get_accounts() -> Result<Vec<AccountInfo>, String> {
    let home_dir = dirs::home_dir().ok_or("Could not find Home Directory")?;
    let file = home_dir.join(".lunarclient").join("settings").join("game").join("accounts.json");
    if !file.exists() { return Ok(vec![]); }

    let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let parsed: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let active_id = parsed["activeAccountLocalId"].as_str().unwrap_or("");
    let mut accounts_list = Vec::new();
    
    if let Some(accounts_map) = parsed["accounts"].as_object() {
        for (local_id, acc_data) in accounts_map {
            accounts_list.push(AccountInfo {
                local_id: local_id.clone(),
                username: acc_data["username"].as_str().unwrap_or("Unknown").to_string(),
                uuid: acc_data["minecraftProfile"]["id"].as_str().unwrap_or("Unknown").to_string(),
                is_active: local_id == active_id,
            });
        }
    }
    Ok(accounts_list)
}

#[tauri::command]
fn set_active_account(local_id: String) -> Result<String, String> {
    let home_dir = dirs::home_dir().unwrap();
    let file = home_dir.join(".lunarclient").join("settings").join("game").join("accounts.json");
    let mut parsed: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    parsed["activeAccountLocalId"] = json!(local_id);
    fs::write(&file, serde_json::to_string_pretty(&parsed).unwrap()).unwrap();
    Ok("Success".to_string())
}

#[tauri::command]
fn remove_account(local_id: String) -> Result<String, String> {
    let home_dir = dirs::home_dir().unwrap();
    let file = home_dir.join(".lunarclient").join("settings").join("game").join("accounts.json");
    let mut parsed: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    if let Some(accounts) = parsed["accounts"].as_object_mut() { accounts.remove(&local_id); }
    if parsed["activeAccountLocalId"] == json!(local_id) { parsed["activeAccountLocalId"] = json!(""); }
    fs::write(&file, serde_json::to_string_pretty(&parsed).unwrap()).unwrap();
    Ok("Removed".to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&String::from("--refresh")) {
        background_refresh();
        return; 
    }

    tauri::Builder::default()
        // Registered remove_patches here!
        .invoke_handler(tauri::generate_handler![add_new_account, get_accounts, set_active_account, remove_account, remove_patches])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}