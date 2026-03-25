use crate::state::AppConfig;
use crate::state::AppState;
use std::fs;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().unwrap();
    Ok(config.clone())
}

#[tauri::command]
pub async fn update_settings(
    config: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Update state
    {
        let mut current_config = state.config.write().unwrap();
        *current_config = config.clone();
    }

    // Save to file
    save_config_to_file(&config)?;

    Ok(())
}

fn save_config_to_file(config: &AppConfig) -> Result<(), String> {
    let config_dir = dirs::home_dir()
        .ok_or("Cannot find home directory")?
        .join(".noteforge");

    fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

    let config_path = config_dir.join("config.toml");
    let toml_str = toml::to_string(config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(config_path, toml_str).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}
