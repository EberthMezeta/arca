//! Diálogos de archivo nativos (portal de escritorio en Linux, Win32, Cocoa).

use rfd::AsyncFileDialog;

use crate::presentation::ports::{ChosenPath, FileDialogs};

pub struct NativeFileDialogs;

impl FileDialogs for NativeFileDialogs {
    fn choose_file_to_open(&self, dialog_title: &str) -> ChosenPath {
        let file_dialog = AsyncFileDialog::new().set_title(dialog_title);
        Box::pin(async move { Some(file_dialog.pick_file().await?.path().to_path_buf()) })
    }

    fn choose_file_to_save(&self, dialog_title: &str, suggested_file_name: &str) -> ChosenPath {
        let file_dialog = AsyncFileDialog::new().set_title(dialog_title).set_file_name(suggested_file_name);
        Box::pin(async move { Some(file_dialog.save_file().await?.path().to_path_buf()) })
    }
}
