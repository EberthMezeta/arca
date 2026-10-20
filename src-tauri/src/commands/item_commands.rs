use tauri::State;
use uuid::Uuid;
use vault_core::application::analyze_password_health::PasswordHealthReport;
use vault_core::application::save_vault_item::SaveVaultItemRequest;
use vault_core::application::search_vault_items::{VaultItemFilter, VaultItemSummary, VaultOverview};
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::totp_configuration::TotpCode;
use vault_core::domain::vault_item::{RevealableField, VaultItem};

use crate::app_state::AppState;
use crate::command_error::CommandResult;

#[tauri::command]
pub fn search_vault_items(
    app_state: State<'_, AppState>,
    item_filter: VaultItemFilter,
) -> CommandResult<Vec<VaultItemSummary>> {
    Ok(app_state.lock_vault_service()?.search_items(&item_filter)?)
}

#[tauri::command]
pub fn get_vault_overview(app_state: State<'_, AppState>) -> CommandResult<VaultOverview> {
    Ok(app_state.lock_vault_service()?.vault_overview()?)
}

#[tauri::command]
pub fn get_vault_item_details(app_state: State<'_, AppState>, item_id: Uuid) -> CommandResult<VaultItem> {
    Ok(app_state.lock_vault_service()?.get_item_details(item_id)?)
}

/// Elemento completo, solo para el editor.
#[tauri::command]
pub fn get_vault_item(app_state: State<'_, AppState>, item_id: Uuid) -> CommandResult<VaultItem> {
    Ok(app_state.lock_vault_service()?.get_item(item_id)?)
}

#[tauri::command]
pub fn save_vault_item(app_state: State<'_, AppState>, save_request: SaveVaultItemRequest) -> CommandResult<VaultItem> {
    Ok(app_state.lock_vault_service()?.save_item(save_request)?)
}

#[tauri::command]
pub fn move_item_to_trash(app_state: State<'_, AppState>, item_id: Uuid) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.move_item_to_trash(item_id)?)
}

#[tauri::command]
pub fn restore_item_from_trash(app_state: State<'_, AppState>, item_id: Uuid) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.restore_item_from_trash(item_id)?)
}

#[tauri::command]
pub fn delete_item_permanently(app_state: State<'_, AppState>, item_id: Uuid) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.delete_item_permanently(item_id)?)
}

#[tauri::command]
pub fn empty_trash(app_state: State<'_, AppState>) -> CommandResult<usize> {
    Ok(app_state.lock_vault_service()?.empty_trash()?)
}

#[tauri::command]
pub fn reveal_item_field(
    app_state: State<'_, AppState>,
    item_id: Uuid,
    requested_field: RevealableField,
) -> CommandResult<SecretText> {
    Ok(app_state.lock_vault_service()?.reveal_item_field(item_id, requested_field)?)
}

#[tauri::command]
pub fn get_totp_code(app_state: State<'_, AppState>, item_id: Uuid) -> CommandResult<TotpCode> {
    Ok(app_state.lock_vault_service()?.current_totp_code(item_id)?)
}

#[tauri::command]
pub fn get_password_health_report(app_state: State<'_, AppState>) -> CommandResult<PasswordHealthReport> {
    Ok(app_state.lock_vault_service()?.password_health_report()?)
}
