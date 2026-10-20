use tauri::State;
use vault_core::domain::vault_preferences::VaultPreferences;

use crate::app_state::AppState;
use crate::command_error::CommandResult;

#[tauri::command]
pub fn get_vault_preferences(app_state: State<'_, AppState>) -> CommandResult<VaultPreferences> {
    Ok(app_state.lock_vault_service()?.preferences()?)
}

#[tauri::command]
pub fn update_vault_preferences(
    app_state: State<'_, AppState>,
    new_preferences: VaultPreferences,
) -> CommandResult<VaultPreferences> {
    Ok(app_state.lock_vault_service()?.update_preferences(new_preferences)?)
}
