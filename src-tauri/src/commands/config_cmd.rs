use tauri::State;

use crate::error::Error;
use crate::model::AppConfig;
use crate::store::ConfigStore;

/// Load the persisted configuration (or defaults if none exists).
#[tauri::command]
pub fn load_config(store: State<'_, ConfigStore>) -> Result<AppConfig, Error> {
    store.load()
}

/// Validate and persist the configuration.
#[tauri::command]
pub fn save_config(store: State<'_, ConfigStore>, config: AppConfig) -> Result<(), Error> {
    store.save(&config)
}
