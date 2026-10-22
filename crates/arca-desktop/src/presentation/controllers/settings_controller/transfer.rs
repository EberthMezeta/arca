//! Importar y exportar: el archivo se lee y se descifra sin tomar la bóveda, y una exportación
//! en claro pendiente se descarta al bloquear.

use std::rc::Rc;
use std::sync::Arc;

use chrono::Local;
use slint::{ComponentHandle, SharedString};
use vault_core::application::import_export_vault::{ExportedFile, ImportLabels, parse_import_file};
use vault_core::domain::ports::vault_exporter::ExportFormat;
use vault_core::domain::ports::vault_importer::ImportFormat;
use vault_core::domain::secret_text::SecretText;

use crate::composition_root::MAXIMUM_IMPORT_FILE_SIZE_BYTES;
use crate::presentation::app_context::AppEvent;
use crate::presentation::background_task::{run_in_background, run_with_vault_in_background, spawn_ui_task};
use crate::presentation::controllers::clipboard_feedback::report_error;
use crate::ui_error::UiError;
use crate::Settings;

use super::SettingsController;
use vault_core::application::vault_service::VaultTransfer;

/// Estado de una exportación (patrón State). Solo `AwaitingDestination` guarda bytes, y bloquear
/// la bóveda vuelve siempre a `Idle`: no hay forma de olvidar una exportación en claro en memoria.
#[derive(Default)]
pub(super) enum ExportState {
    #[default]
    Idle,
    /// Generada; espera a que el usuario elija dónde guardarla.
    AwaitingDestination(ExportedFile),
}

impl ExportState {
    /// Saca la exportación pendiente (si la hay) y vuelve a `Idle`.
    pub(super) fn take_pending(&mut self) -> Option<ExportedFile> {
        match std::mem::take(self) {
            Self::AwaitingDestination(exported_file) => Some(exported_file),
            Self::Idle => None,
        }
    }
}

pub(super) struct TransferFormatDescription<Format> {
    transfer_format: Format,
    pub(super) label_key: &'static str,
    pub(super) hint_key: &'static str,
}

pub(super) const IMPORT_FORMATS: [TransferFormatDescription<ImportFormat>; 4] = [
    TransferFormatDescription {
        transfer_format: ImportFormat::BitwardenJson,
        label_key: "transfer.formats.bitwardenJson",
        hint_key: "transfer.importHints.bitwardenJson",
    },
    TransferFormatDescription {
        transfer_format: ImportFormat::BitwardenCsv,
        label_key: "transfer.formats.bitwardenCsv",
        hint_key: "transfer.importHints.bitwardenCsv",
    },
    TransferFormatDescription {
        transfer_format: ImportFormat::BrowserCsv,
        label_key: "transfer.formats.browserCsv",
        hint_key: "transfer.importHints.browserCsv",
    },
    TransferFormatDescription {
        transfer_format: ImportFormat::EncryptedVaultExport,
        label_key: "transfer.formats.encryptedVaultExport",
        hint_key: "transfer.importHints.encryptedVaultExport",
    },
];

pub(super) const EXPORT_FORMATS: [TransferFormatDescription<ExportFormat>; 3] = [
    TransferFormatDescription {
        transfer_format: ExportFormat::EncryptedVaultExport,
        label_key: "transfer.formats.encryptedVaultExport",
        hint_key: "",
    },
    TransferFormatDescription {
        transfer_format: ExportFormat::BitwardenJson,
        label_key: "transfer.formats.bitwardenJson",
        hint_key: "",
    },
    TransferFormatDescription {
        transfer_format: ExportFormat::BitwardenCsv,
        label_key: "transfer.formats.bitwardenCsv",
        hint_key: "",
    },
];

impl SettingsController {
    pub(super) fn select_import_format(&self, format_position: i32) {
        let Some(description) = usize::try_from(format_position).ok().and_then(|position| IMPORT_FORMATS.get(position)) else {
            return;
        };
        self.with_settings(|settings| {
            settings.set_import_format_index(format_position);
            settings.set_import_hint(self.context.text(description.hint_key).into());
            settings.set_import_requires_password(description.transfer_format.traits().requires_file_password);
        });
    }

    pub(super) fn import_file(self: &Rc<Self>) {
        self.context.with_window(|app_window| {
            let settings = app_window.global::<Settings>();
            let Some(description) = usize::try_from(settings.get_import_format_index()).ok().and_then(|position| IMPORT_FORMATS.get(position))
            else {
                return;
            };
            if settings.get_is_importing() {
                return;
            }
            let import_format = description.transfer_format;
            let file_password = import_format
                .traits()
                .requires_file_password
                .then(|| SecretText::new(settings.get_import_file_password().as_str()));
            settings.set_is_importing(true);

            // Lo que la importación escribe en la bóveda, en el idioma de la interfaz.
            let import_labels = ImportLabels {
                untitled_item_title: self.context.text("transfer.untitledItem"),
                invalid_totp_field_label: self.context.text("transfer.invalidTotpField"),
            };
            let controller = Rc::clone(self);
            spawn_ui_task(async move {
                let dialog_title = controller.context.text("transfer.chooseImportFile");
                let Some(chosen_path) = controller.context.file_dialogs.choose_file_to_open(&dialog_title).await else {
                    return controller.with_settings(|settings| settings.set_is_importing(false));
                };
                // Leer y descifrar el archivo (una copia `.arca` puede pedir un Argon2id de decenas de
                // segundos) se hace sin tomar la bóveda: así el bloqueo nunca tiene que esperar.
                let transfer_formats = controller.context.shared_vault.lock_service().map(|vault_service| vault_service.transfer_format_registry());
                let user_files = Arc::clone(&controller.context.user_files);
                let parse_result = match transfer_formats {
                    Ok(transfer_formats) => {
                        run_in_background(move || {
                            let file_bytes =
                                user_files.read_limited(&chosen_path, MAXIMUM_IMPORT_FILE_SIZE_BYTES, UiError::import_file_too_large())?;
                            Ok(parse_import_file(&transfer_formats, import_format, &file_bytes, file_password.as_ref())?)
                        })
                        .await
                    }
                    Err(ui_error) => Err(ui_error),
                };
                let import_result = match parse_result {
                    Ok(imported_vault_data) => {
                        run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                            Ok(vault_service.merge_imported_data(imported_vault_data, &import_labels)?)
                        })
                        .await
                    }
                    Err(ui_error) => Err(ui_error),
                };
                controller.with_settings(|settings| settings.set_is_importing(false));
                match import_result {
                    Ok(import_summary) => {
                        controller.with_settings(|settings| settings.set_import_file_password(SharedString::new()));
                        controller.context.events.publish(AppEvent::VaultDataChanged);
                        let imported_count = i64::try_from(import_summary.imported_item_count).unwrap_or(i64::MAX);
                        let folder_count = import_summary.created_folder_count.to_string();
                        controller.context.toasts.success(controller.context.count_text(
                            "transfer.imported",
                            imported_count,
                            &[("folderCount", &folder_count)],
                        ));
                    }
                    Err(ui_error) => report_error(&controller.context, &ui_error),
                }
            });
        });
    }

    pub(super) fn select_export_format(&self, format_position: i32) {
        let Some(description) = usize::try_from(format_position).ok().and_then(|position| EXPORT_FORMATS.get(position)) else {
            return;
        };
        self.with_settings(|settings| {
            settings.set_export_format_index(format_position);
            settings.set_export_requires_password(description.transfer_format.traits().requires_file_password);
            settings.set_export_is_plaintext(description.transfer_format.traits().is_plaintext);
            settings.set_plaintext_risk_acknowledged(false);
        });
    }

    /// Primero se comprueba la contraseña maestra (y se cifra); luego se pregunta dónde guardar.
    pub(super) fn export_file(self: &Rc<Self>) {
        self.context.with_window(|app_window| {
            let settings = app_window.global::<Settings>();
            let Some(description) = usize::try_from(settings.get_export_format_index()).ok().and_then(|position| EXPORT_FORMATS.get(position))
            else {
                return;
            };
            if settings.get_is_exporting() {
                return;
            }
            let export_format = description.transfer_format;
            let master_password = SecretText::new(settings.get_export_master_password().as_str());
            let file_password = export_format
                .traits()
                .requires_file_password
                .then(|| SecretText::new(settings.get_export_file_password().as_str()));
            settings.set_export_master_password(SharedString::new());
            settings.set_is_exporting(true);

            let controller = Rc::clone(self);
            spawn_ui_task(async move {
                let export_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                    Ok(vault_service.export_vault(&master_password, export_format, file_password.as_ref())?)
                })
                .await;
                let exported_file = match export_result {
                    Ok(exported_file) => exported_file,
                    Err(ui_error) => {
                        controller.with_settings(|settings| settings.set_is_exporting(false));
                        return report_error(&controller.context, &ui_error);
                    }
                };
                let suggested_file_name =
                    format!("arca-export-{}.{}", Local::now().format("%Y-%m-%d"), exported_file.suggested_file_extension);
                *controller.export_state.borrow_mut() = ExportState::AwaitingDestination(exported_file);
                let dialog_title = controller.context.text("transfer.chooseExportFile");
                let destination_path = controller.context.file_dialogs.choose_file_to_save(&dialog_title, &suggested_file_name).await;
                let pending_export = controller.export_state.borrow_mut().take_pending();
                let (Some(destination_path), Some(exported_file)) = (destination_path, pending_export) else {
                    return controller.with_settings(|settings| settings.set_is_exporting(false));
                };
                let user_files = Arc::clone(&controller.context.user_files);
                let write_result = run_in_background(move || {
                    user_files.write_private(&destination_path, &exported_file.file_bytes)
                })
                .await;
                controller.with_settings(|settings| settings.set_is_exporting(false));
                match write_result {
                    Ok(()) => {
                        controller.with_settings(|settings| {
                            settings.set_export_file_password(SharedString::new());
                            settings.set_export_confirmed_password(SharedString::new());
                            settings.set_plaintext_risk_acknowledged(false);
                        });
                        controller.report_success("transfer.exported");
                    }
                    Err(ui_error) => report_error(&controller.context, &ui_error),
                }
            });
        });
    }
}
