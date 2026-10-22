//! Diálogos de archivo nativos (portal de escritorio en Linux, Win32, Cocoa).
//! Son asíncronos para que la ventana siga dibujándose mientras están abiertos.

use std::path::PathBuf;

use rfd::AsyncFileDialog;

pub struct FileTypeFilter<'filter> {
    pub filter_label: &'filter str,
    pub file_extensions: &'filter [&'filter str],
}

pub async fn ask_file_to_open(dialog_title: &str, file_type_filter: Option<FileTypeFilter<'_>>) -> Option<PathBuf> {
    let mut file_dialog = AsyncFileDialog::new().set_title(dialog_title);
    if let Some(file_type_filter) = file_type_filter {
        file_dialog = file_dialog.add_filter(file_type_filter.filter_label, file_type_filter.file_extensions);
    }
    Some(file_dialog.pick_file().await?.path().to_path_buf())
}

pub async fn ask_file_to_save(dialog_title: &str, suggested_file_name: &str) -> Option<PathBuf> {
    let selected_file = AsyncFileDialog::new().set_title(dialog_title).set_file_name(suggested_file_name).save_file().await?;
    Some(selected_file.path().to_path_buf())
}
