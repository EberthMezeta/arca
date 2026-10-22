//! Arca: gestor de contraseñas local. Interfaz nativa con Slint sobre `vault-core`.

// Sin consola en Windows al abrir la app con doble clic.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod composition_root;
mod platform;
mod presentation;
mod shared_vault;
mod ui_error;
mod vault_events;

use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use rfd::{MessageDialog, MessageLevel};
use slint::ComponentHandle;

use crate::composition_root::{ComposedApplication, StartupOutcome, compose_application};
use crate::platform::file_dialogs::NativeFileDialogs;
use crate::platform::local_user_files::LocalUserFiles;
use crate::platform::{auto_lock_scheduler, process_hardening, system_color_scheme, system_lock_watcher};
use crate::presentation::app_context::{AppContext, SystemPorts};
use crate::presentation::{controllers, vault_event_bridge};
use crate::shared_vault::VaultLocker;

/// Código generado por Slint a partir de `ui/*.slint`: no sigue las reglas de estilo del proyecto.
#[allow(clippy::unwrap_used, clippy::pedantic)]
mod generated_ui {
    slint::include_modules!();
}
use generated_ui::*;

/// Aún no se conoce el idioma elegido (se carga después): el aviso va en los dos idiomas.
/// También se muestra si la versión anterior (`RustVault`) sigue abierta: hay que cerrarla para migrar sus datos.
const ANOTHER_INSTANCE_MESSAGE: &str = "Arca (o RustVault, su versión anterior) ya está abierta en otra ventana. Ciérrala e inténtalo de nuevo.\n\n\
     Arca (or RustVault, its previous version) is already open in another window. Close it and try again.";

fn warn_about_running_instance() {
    MessageDialog::new()
        .set_title("Arca")
        .set_description(ANOTHER_INSTANCE_MESSAGE)
        .set_level(MessageLevel::Warning)
        .show();
}

fn main() -> ExitCode {
    // El portal de escritorio deja de responder en cuanto el proceso se endurece.
    let system_prefers_dark = system_color_scheme::read_current();
    process_hardening::apply();

    let vault_event_sink = vault_event_bridge::event_sink_to_ui_thread();
    let composed_application = match compose_application(vault_event_sink.clone()) {
        Ok(StartupOutcome::Ready(composed_application)) => composed_application,
        Ok(StartupOutcome::AnotherInstanceIsRunning) => {
            warn_about_running_instance();
            return ExitCode::SUCCESS;
        }
        Err(startup_error) => {
            eprintln!("No se pudo iniciar la aplicación: {startup_error}");
            return ExitCode::FAILURE;
        }
    };
    let ComposedApplication { shared_vault, vault_file_path, ui_preferences_store, single_instance_lock } =
        *composed_application;

    let app_window = match AppWindow::new() {
        Ok(app_window) => app_window,
        Err(window_error) => {
            eprintln!("No se pudo crear la ventana: {window_error}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(system_is_dark) = system_prefers_dark {
        app_window.global::<Theme>().set_system_is_dark(system_is_dark);
    }

    let vault_locker = VaultLocker::new(shared_vault.clone(), vault_event_sink);
    system_lock_watcher::start(&vault_locker);
    auto_lock_scheduler::start(vault_locker);

    let system_ports = SystemPorts { file_dialogs: Rc::new(NativeFileDialogs), user_files: Arc::new(LocalUserFiles) };
    let context =
        AppContext::new(&app_window, shared_vault.clone(), vault_file_path, ui_preferences_store, system_ports);
    controllers::install_all(&context, &app_window);

    let run_result = app_window.run();

    // Al cerrar la ventana se bloquea la bóveda: borra las claves y vacía el portapapeles si aún tiene un secreto.
    shared_vault.lock_if_unlocked();
    drop(single_instance_lock);
    match run_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(event_loop_error) => {
            eprintln!("La ventana se cerró con un error: {event_loop_error}");
            ExitCode::FAILURE
        }
    }
}
