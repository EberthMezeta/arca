//! Capa de presentación: comandos Tauri finos que validan, delegan en `VaultService` y traducen errores.

mod adapters;
mod app_state;
mod auto_lock_scheduler;
mod bounded_file_reader;
mod command_error;
mod commands;
mod composition_root;
mod file_dialogs;
mod process_hardening;
mod single_instance_lock;
mod system_lock_watcher;
mod vault_events;

use tauri::{App, AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::app_state::AppState;
use crate::composition_root::StartupOutcome;
use crate::vault_events::{VaultLockReason, emit_vault_locked};

const MAIN_WINDOW_LABEL: &str = "main";
/// En Rust no se conoce el idioma elegido en la UI (vive en el `localStorage` del `WebView`).
const ANOTHER_INSTANCE_MESSAGE: &str =
    "RustVault ya está abierto en otra ventana.\nRustVault is already open in another window.";

/// Al cerrar la app se bloquea la bóveda: borra las claves y vacía el portapapeles si aún tiene un secreto.
fn lock_on_exit(app_handle: &AppHandle, run_event: RunEvent) {
    if !matches!(run_event, RunEvent::Exit) {
        return;
    }
    if let Some(app_state) = app_handle.try_state::<AppState>() {
        app_state.lock_if_unlocked();
    }
}

/// Segunda instancia: no toca la bóveda; avisa y se cierra.
fn warn_about_running_instance_and_exit(tauri_app: &App) {
    if let Some(main_window) = tauri_app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = main_window.hide();
    }
    let app_handle = tauri_app.handle().clone();
    tauri_app
        .dialog()
        .message(ANOTHER_INSTANCE_MESSAGE)
        .title("RustVault")
        .kind(MessageDialogKind::Warning)
        .show(move |_| app_handle.exit(0));
}

pub fn run() {
    process_hardening::apply();

    let build_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|tauri_app| {
            let app_handle = tauri_app.handle().clone();
            let StartupOutcome::Ready(composed_application) = composition_root::compose_application(&app_handle)? else {
                warn_about_running_instance_and_exit(tauri_app);
                return Ok(());
            };
            tauri_app.manage(AppState::new(*composed_application));
            system_lock_watcher::start(&app_handle);
            auto_lock_scheduler::start(app_handle);
            Ok(())
        })
        .on_window_event(|window, window_event| {
            let should_check_minimize = matches!(window_event, WindowEvent::Resized(_));
            if !should_check_minimize || !window.is_minimized().unwrap_or(false) {
                return;
            }
            let Some(app_state) = window.try_state::<AppState>() else {
                return;
            };
            if app_state.lock_if_minimize_policy_applies() {
                emit_vault_locked(window.app_handle(), VaultLockReason::WindowMinimized);
            }
        })
        .invoke_handler(commands::all_command_handlers())
        .build(tauri::generate_context!());

    match build_result {
        Ok(tauri_app) => tauri_app.run(lock_on_exit),
        Err(startup_error) => {
            eprintln!("No se pudo iniciar la aplicación: {startup_error}");
            std::process::exit(1);
        }
    }
}
