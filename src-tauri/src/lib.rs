//! Capa de presentación: comandos Tauri finos que validan, delegan en `VaultService` y traducen errores.

mod adapters;
mod app_state;
mod auto_lock_scheduler;
mod command_error;
mod commands;
mod composition_root;
mod file_dialogs;
mod system_lock_watcher;
mod vault_events;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

use crate::app_state::AppState;
use crate::vault_events::{VaultLockReason, emit_vault_locked};

/// Al cerrar la app se bloquea la bóveda: borra las claves y vacía el portapapeles si aún tiene un secreto.
fn lock_on_exit(app_handle: &AppHandle, run_event: RunEvent) {
    if !matches!(run_event, RunEvent::Exit) {
        return;
    }
    app_handle.state::<AppState>().lock_if_unlocked();
}

pub fn run() {
    let build_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|tauri_app| {
            let app_handle = tauri_app.handle().clone();
            let composed_application = composition_root::compose_application(&app_handle)?;
            tauri_app.manage(AppState::new(composed_application));
            system_lock_watcher::start(&app_handle);
            auto_lock_scheduler::start(app_handle);
            Ok(())
        })
        .on_window_event(|window, window_event| {
            let should_check_minimize = matches!(window_event, WindowEvent::Resized(_));
            if !should_check_minimize || !window.is_minimized().unwrap_or(false) {
                return;
            }
            let app_state = window.state::<AppState>();
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
