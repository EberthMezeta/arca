use std::fs;

use tauri::{AppHandle, State};
use uuid::Uuid;
use vault_core::domain::vault_item::AttachmentReference;
use vault_core::infrastructure::storage::owner_only_files::write_owner_only_file;
use zeroize::Zeroizing;

use crate::app_state::AppState;
use crate::command_error::{CommandError, CommandResult};
use crate::file_dialogs::{ask_file_to_open, ask_file_to_save};

/// Devuelve `None` si el usuario cancela el diálogo.
#[tauri::command]
pub async fn add_attachment_from_file(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    item_id: Uuid,
    dialog_title: String,
) -> CommandResult<Option<AttachmentReference>> {
    let Some(selected_file_path) = ask_file_to_open(&app_handle, &dialog_title, None) else {
        return Ok(None);
    };
    let file_bytes = Zeroizing::new(fs::read(&selected_file_path).map_err(CommandError::file_access)?);
    let file_name = selected_file_path.file_name().map(|file_name| file_name.to_string_lossy().into_owned()).unwrap_or_default();

    let attachment_reference = app_state.lock_vault_service()?.add_attachment(item_id, &file_name, &file_bytes)?;
    Ok(Some(attachment_reference))
}

/// Descifra el adjunto y lo guarda donde elija el usuario. `false` si cancela.
#[tauri::command]
pub async fn save_attachment_to_file(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    item_id: Uuid,
    attachment_id: Uuid,
    dialog_title: String,
) -> CommandResult<bool> {
    let decrypted_attachment = app_state.lock_vault_service()?.read_attachment(item_id, attachment_id)?;
    let Some(destination_path) = ask_file_to_save(&app_handle, &dialog_title, &decrypted_attachment.file_name) else {
        return Ok(false);
    };
    write_owner_only_file(&destination_path, decrypted_attachment.file_bytes.as_slice()).map_err(CommandError::file_write)?;
    Ok(true)
}

#[tauri::command]
pub fn delete_attachment(app_state: State<'_, AppState>, item_id: Uuid, attachment_id: Uuid) -> CommandResult<()> {
    Ok(app_state.lock_vault_service()?.delete_attachment(item_id, attachment_id)?)
}
