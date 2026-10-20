//! Los diálogos de archivo se abren desde Rust: el frontend nunca envía rutas,
//! así un script inyectado no podría leer ni sobrescribir archivos arbitrarios.

use std::path::PathBuf;

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

pub struct FileTypeFilter<'filter> {
    pub filter_label: &'filter str,
    pub file_extensions: &'filter [&'filter str],
}

pub fn ask_file_to_open(app_handle: &AppHandle, dialog_title: &str, file_type_filter: Option<FileTypeFilter<'_>>) -> Option<PathBuf> {
    let mut file_dialog = app_handle.dialog().file().set_title(dialog_title);
    if let Some(file_type_filter) = file_type_filter {
        file_dialog = file_dialog.add_filter(file_type_filter.filter_label, file_type_filter.file_extensions);
    }
    file_dialog.blocking_pick_file()?.into_path().ok()
}

pub fn ask_file_to_save(app_handle: &AppHandle, dialog_title: &str, suggested_file_name: &str) -> Option<PathBuf> {
    app_handle
        .dialog()
        .file()
        .set_title(dialog_title)
        .set_file_name(suggested_file_name)
        .blocking_save_file()?
        .into_path()
        .ok()
}
