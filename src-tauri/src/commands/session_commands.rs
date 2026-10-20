use tauri::State;
use vault_core::application::vault_service::VaultStatus;
use vault_core::domain::secret_text::SecretText;

use crate::app_state::AppState;
use crate::command_error::CommandResult;

#[tauri::command]
pub fn get_vault_status(app_state: State<'_, AppState>) -> CommandResult<VaultStatus> {
    Ok(app_state.lock_vault_service()?.vault_status())
}

/// Argon2id tarda ~1 s: los comandos costosos son `async` para no congelar la ventana.
#[tauri::command]
pub async fn create_vault(app_state: State<'_, AppState>, master_password: SecretText) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.create_vault(&master_password)?)
}

#[tauri::command]
pub async fn unlock_vault(app_state: State<'_, AppState>, master_password: SecretText) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.unlock_vault(&master_password)?)
}

#[tauri::command]
/// No puede fallar: bloquear funciona incluso con el estado envenenado.
pub fn lock_vault(app_state: State<'_, AppState>) {
    app_state.lock_if_unlocked();
}

#[tauri::command]
pub fn register_user_activity(app_state: State<'_, AppState>) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.register_user_activity()?)
}

#[tauri::command]
pub async fn change_master_password(
    app_state: State<'_, AppState>,
    current_master_password: SecretText,
    new_master_password: SecretText,
) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.change_master_password(&current_master_password, &new_master_password)?)
}

#[tauri::command]
pub fn get_vault_location(app_state: State<'_, AppState>) -> String {
    app_state.vault_file_path.display().to_string()
}
