use std::fs;

use tauri::{AppHandle, State};
use vault_core::application::import_export_vault::ImportSummary;
use vault_core::domain::ports::vault_exporter::ExportFormat;
use vault_core::domain::ports::vault_importer::ImportFormat;
use vault_core::domain::secret_text::SecretText;
use vault_core::infrastructure::storage::owner_only_files::write_owner_only_file;
use zeroize::Zeroizing;

use crate::app_state::AppState;
use crate::command_error::{CommandError, CommandResult};
use crate::file_dialogs::{FileTypeFilter, ask_file_to_open, ask_file_to_save};

fn accepted_extensions(import_format: ImportFormat) -> &'static [&'static str] {
    match import_format {
        ImportFormat::BitwardenJson => &["json"],
        ImportFormat::BitwardenCsv | ImportFormat::BrowserCsv => &["csv"],
        ImportFormat::EncryptedVaultExport => &["rvexport", "json"],
    }
}

/// `None` si el usuario cancela el diálogo.
#[tauri::command]
pub async fn import_vault_file(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    import_format: ImportFormat,
    file_password: Option<SecretText>,
    dialog_title: String,
) -> CommandResult<Option<ImportSummary>> {
    let file_type_filter = FileTypeFilter { filter_label: &dialog_title, file_extensions: accepted_extensions(import_format) };
    let Some(selected_file_path) = ask_file_to_open(&app_handle, &dialog_title, Some(file_type_filter)) else {
        return Ok(None);
    };
    let file_bytes = Zeroizing::new(fs::read(selected_file_path).map_err(CommandError::file_access)?);

    let import_summary =
        app_state.lock_vault_service()?.import_file(import_format, &file_bytes, file_password.as_ref())?;
    Ok(Some(import_summary))
}

/// `false` si el usuario cancela el diálogo.
#[tauri::command]
pub async fn export_vault_to_file(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    export_format: ExportFormat,
    file_password: Option<SecretText>,
    dialog_title: String,
) -> CommandResult<bool> {
    let exported_file = app_state.lock_vault_service()?.export_vault(export_format, file_password.as_ref())?;
    let exported_bytes = Zeroizing::new(exported_file.file_bytes);
    let suggested_file_name = format!("rust-vault-export.{}", exported_file.suggested_file_extension);

    let Some(destination_path) = ask_file_to_save(&app_handle, &dialog_title, &suggested_file_name) else {
        return Ok(false);
    };
    // Puede ir en texto plano (CSV / JSON de Bitwarden): solo el usuario debe poder leerlo.
    write_owner_only_file(&destination_path, exported_bytes.as_slice()).map_err(CommandError::file_write)?;
    Ok(true)
}
