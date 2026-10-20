use tauri::State;
use uuid::Uuid;
use vault_core::domain::folder::Folder;

use crate::app_state::AppState;
use crate::command_error::CommandResult;

#[tauri::command]
pub fn list_folders(app_state: State<'_, AppState>) -> CommandResult<Vec<Folder>> {
    Ok(app_state.lock_vault_service()?.list_folders()?)
}

#[tauri::command]
pub fn create_folder(app_state: State<'_, AppState>, folder_name: String) -> CommandResult<Folder> {
    Ok(app_state.lock_vault_service()?.create_folder(&folder_name)?)
}

#[tauri::command]
pub fn rename_folder(app_state: State<'_, AppState>, folder_id: Uuid, folder_name: String) -> CommandResult<Folder> {
    Ok(app_state.lock_vault_service()?.rename_folder(folder_id, &folder_name)?)
}

#[tauri::command]
pub fn delete_folder(app_state: State<'_, AppState>, folder_id: Uuid) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.delete_folder(folder_id)?)
}
