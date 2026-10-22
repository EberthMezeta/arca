//! Tests de los controladores con una ventana Slint sin pantalla, una bóveda en memoria
//! (Argon2id barato) y dobles de los diálogos y archivos del usuario.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use slint::{ComponentHandle, Model, SharedString};
use vault_core::application::search_vault_items::VaultItemFilter;
use vault_core::application::test_support::{STRONG_MASTER_PASSWORD, build_test_vault_service};
use vault_core::application::vault_service::{VaultItems, VaultSession};
use vault_core::domain::item_content::ItemContentType;
use vault_core::domain::secret_text::SecretText;
use zeroize::Zeroizing;

use crate::platform::ui_preferences_store::UiPreferencesStore;
use crate::presentation::app_commands::AppCommand;
use crate::presentation::app_context::{AppContext, AppEvent, SystemPorts};
use crate::presentation::controllers;
use crate::presentation::ports::{ChosenPath, FileDialogs, UserFiles};
use crate::shared_vault::SharedVault;
use crate::ui_error::{UiError, UiResult};
use crate::{AppWindow, CommandPalette, ItemDetails, ItemEditor, ItemPanel, Settings, Toasts, Workspace};

thread_local! {
    /// Slint admite un backend por hilo, y cada test corre en su propio hilo.
    static SLINT_TESTING_BACKEND_READY: Cell<bool> = const { Cell::new(false) };
}

#[derive(Default)]
struct FakeFileDialogs {
    chosen_path: RefCell<Option<PathBuf>>,
}

impl FileDialogs for FakeFileDialogs {
    fn choose_file_to_open(&self, _dialog_title: &str) -> ChosenPath {
        Box::pin(future::ready(self.chosen_path.borrow().clone()))
    }

    fn choose_file_to_save(&self, _dialog_title: &str, _suggested_file_name: &str) -> ChosenPath {
        Box::pin(future::ready(self.chosen_path.borrow().clone()))
    }
}

#[derive(Default)]
struct FakeUserFiles {
    files_by_path: Mutex<HashMap<PathBuf, Vec<u8>>>,
}

impl UserFiles for FakeUserFiles {
    fn read_limited(
        &self,
        file_path: &Path,
        maximum_byte_count: u64,
        too_large_error: UiError,
    ) -> UiResult<Zeroizing<Vec<u8>>> {
        let files_by_path = self.files_by_path.lock().expect("mutex");
        let file_bytes = files_by_path.get(file_path).ok_or_else(|| UiError::file_access("no existe"))?;
        if file_bytes.len() as u64 > maximum_byte_count {
            return Err(too_large_error);
        }
        Ok(Zeroizing::new(file_bytes.clone()))
    }

    fn write_private(&self, file_path: &Path, file_bytes: &[u8]) -> UiResult<()> {
        self.files_by_path.lock().expect("mutex").insert(file_path.to_path_buf(), file_bytes.to_vec());
        Ok(())
    }
}

struct UnlockedApp {
    app_window: AppWindow,
    context: Rc<AppContext>,
}

/// La app con todos los controladores instalados y una bóveda vacía ya desbloqueada.
fn unlocked_app() -> UnlockedApp {
    SLINT_TESTING_BACKEND_READY.with(|backend_ready| {
        if !backend_ready.replace(true) {
            i_slint_backend_testing::init_no_event_loop();
        }
    });
    let app_window = AppWindow::new().expect("ventana");
    let mut vault_service = build_test_vault_service().vault_service;
    vault_service.create_vault(&SecretText::new(STRONG_MASTER_PASSWORD)).expect("crear bóveda");
    let preferences_path = std::env::temp_dir().join(format!(
        "arca-controller-tests-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    let system_ports = SystemPorts {
        file_dialogs: Rc::new(FakeFileDialogs::default()),
        user_files: Arc::new(FakeUserFiles::default()),
    };
    let context = AppContext::new(
        &app_window,
        SharedVault::new(vault_service),
        PathBuf::from("personal.vault"),
        UiPreferencesStore::new(preferences_path),
        system_ports,
    );
    controllers::install_all(&context, &app_window);
    context.events.publish(AppEvent::SessionUnlocked);
    UnlockedApp { app_window, context }
}

impl UnlockedApp {
    /// Crea un inicio de sesión rellenando el editor como lo haría el usuario.
    fn create_login(&self, title: &str, password: &str) {
        AppCommand::CreateItem(ItemContentType::Login).execute(&self.context);
        let item_editor = self.app_window.global::<ItemEditor>();
        item_editor.set_title(title.into());
        item_editor.set_password(password.into());
        item_editor.invoke_save();
    }

    fn stored_titles(&self, item_filter: &VaultItemFilter) -> Vec<String> {
        let vault_service = self.context.shared_vault.lock_service().expect("bóveda");
        vault_service.search_items(item_filter).expect("buscar").into_iter().map(|summary| summary.title).collect()
    }
}

#[test]
fn ctrl_k_opens_the_command_palette_and_unknown_keys_pass_through() {
    let app = unlocked_app();
    let workspace = app.app_window.global::<Workspace>();

    assert!(!workspace.invoke_control_key_shortcut("x".into()));
    assert!(workspace.invoke_control_key_shortcut("k".into()));

    assert!(app.app_window.global::<CommandPalette>().get_is_open());
}

#[test]
fn saving_from_the_editor_stores_the_item_and_opens_its_details() {
    let app = unlocked_app();

    app.create_login("GitHub", "glacier-mantis-orbit-velvet-27");

    assert_eq!(app.stored_titles(&VaultItemFilter::default()), vec!["GitHub".to_string()]);
    assert_eq!(app.app_window.global::<Workspace>().get_item_panel(), ItemPanel::Details);
    assert_eq!(app.app_window.global::<ItemDetails>().get_title(), SharedString::from("GitHub"));
}

#[test]
fn details_show_the_password_masked_until_revealed() {
    let app = unlocked_app();
    app.create_login("Banco", "pw-del-banco-1234");
    let item_details = app.app_window.global::<ItemDetails>();
    let password_row = || {
        item_details
            .get_main_rows()
            .iter()
            .find(|detail_row| detail_row.label == "Contraseña" || detail_row.label == "Password")
            .expect("fila")
    };

    assert_ne!(password_row().value, SharedString::from("pw-del-banco-1234"));
    item_details.invoke_toggle_reveal(password_row().row_index);

    assert_eq!(password_row().value, SharedString::from("pw-del-banco-1234"));
}

#[test]
fn moving_to_trash_offers_an_undo_that_restores_the_item() {
    let app = unlocked_app();
    app.create_login("Correo", "pw-del-correo-5678");

    app.app_window.global::<ItemDetails>().invoke_move_to_trash();
    let trash_filter = VaultItemFilter { should_show_trash: true, ..VaultItemFilter::default() };
    assert_eq!(app.stored_titles(&trash_filter), vec!["Correo".to_string()]);
    let toasts = app.app_window.global::<Toasts>();
    let undo_toast =
        toasts.get_visible_toasts().iter().find(|toast| !toast.action_label.is_empty()).expect("aviso con «Deshacer»");
    toasts.invoke_run_action(undo_toast.toast_id);

    assert!(app.stored_titles(&trash_filter).is_empty());
    assert_eq!(app.stored_titles(&VaultItemFilter::default()), vec!["Correo".to_string()]);
}

#[test]
fn locking_clears_every_password_typed_in_the_forms() {
    let app = unlocked_app();
    let settings = app.app_window.global::<Settings>();
    settings.set_current_master_password("maestra-actual".into());
    settings.set_export_file_password("clave-de-la-copia".into());
    AppCommand::CreateItem(ItemContentType::Login).execute(&app.context);
    app.app_window.global::<ItemEditor>().set_password("a-medio-escribir".into());

    app.context.shared_vault.lock_if_unlocked();
    app.context.events.publish(AppEvent::SessionLocked);

    assert!(settings.get_current_master_password().is_empty());
    assert!(settings.get_export_file_password().is_empty());
    assert!(app.app_window.global::<ItemEditor>().get_password().is_empty());
}

#[test]
fn editing_without_touching_the_masked_password_keeps_it() {
    let app = unlocked_app();
    app.create_login("Correo", "pw-original-9876");
    let item_details = app.app_window.global::<ItemDetails>();

    item_details.invoke_edit();
    let item_editor = app.app_window.global::<ItemEditor>();
    assert_ne!(item_editor.get_password(), SharedString::from("pw-original-9876"), "el editor no recibe el secreto");
    item_editor.set_title("Correo personal".into());
    item_editor.invoke_save();

    let password_row = item_details
        .get_main_rows()
        .iter()
        .find(|detail_row| detail_row.kind == crate::DetailRowKind::Secret)
        .expect("fila");
    item_details.invoke_toggle_reveal(password_row.row_index);
    let revealed_row = item_details
        .get_main_rows()
        .iter()
        .find(|detail_row| detail_row.row_index == password_row.row_index)
        .expect("fila");
    assert_eq!(item_details.get_title(), SharedString::from("Correo personal"));
    assert_eq!(revealed_row.value, SharedString::from("pw-original-9876"));
}
