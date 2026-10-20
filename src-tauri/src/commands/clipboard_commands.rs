use std::sync::MutexGuard;

use serde::Serialize;
use tauri::State;
use uuid::Uuid;
use vault_core::application::vault_service::VaultService;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_item::RevealableField;

use crate::app_state::AppState;
use crate::command_error::CommandResult;

/// La UI usa `clear_after_seconds` para mostrar "Copiado · se borrará en N s".
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardCopyResult {
    clear_after_seconds: u64,
}

fn copy_result(vault_service: &MutexGuard<'_, VaultService>) -> CommandResult<ClipboardCopyResult> {
    Ok(ClipboardCopyResult { clear_after_seconds: vault_service.preferences()?.clipboard_clear_after_seconds })
}

#[tauri::command]
pub fn copy_item_field(
    app_state: State<'_, AppState>,
    item_id: Uuid,
    requested_field: RevealableField,
) -> CommandResult<ClipboardCopyResult> {
    let mut vault_service = app_state.lock_vault_service()?;
    vault_service.copy_item_field_to_clipboard(item_id, requested_field)?;
    copy_result(&vault_service)
}

#[tauri::command]
pub fn copy_totp_code(app_state: State<'_, AppState>, item_id: Uuid) -> CommandResult<ClipboardCopyResult> {
    let mut vault_service = app_state.lock_vault_service()?;
    vault_service.copy_totp_code_to_clipboard(item_id)?;
    copy_result(&vault_service)
}

/// Para contraseñas recién generadas que aún no se han guardado.
#[tauri::command]
pub fn copy_text_secret(app_state: State<'_, AppState>, secret_value: SecretText) -> CommandResult<ClipboardCopyResult> {
    let mut vault_service = app_state.lock_vault_service()?;
    vault_service.copy_secret_to_clipboard(secret_value)?;
    copy_result(&vault_service)
}
