//! Trabajo lento (Argon2id, cifrar adjuntos, leer/escribir archivos) fuera del hilo de la interfaz,
//! para que la ventana siga respondiendo. El resultado vuelve al hilo de la interfaz con `.await`.

use std::future::Future;
use std::thread;

use futures_channel::oneshot;
use vault_core::application::vault_service::VaultService;

use crate::shared_vault::SharedVault;
use crate::ui_error::{UiError, UiResult};

/// Lanza una tarea asíncrona en el bucle de eventos de Slint (diálogos de archivo, tareas en segundo plano).
pub fn spawn_ui_task(ui_task: impl Future<Output = ()> + 'static) {
    let _ = slint::spawn_local(ui_task);
}

/// Si el hilo entra en pánico, el canal se cierra y se informa como error interno.
pub async fn run_in_background<Output: Send + 'static>(
    background_work: impl FnOnce() -> UiResult<Output> + Send + 'static,
) -> UiResult<Output> {
    let (result_sender, result_receiver) = oneshot::channel();
    thread::spawn(move || {
        let _ = result_sender.send(background_work());
    });
    result_receiver.await.unwrap_or_else(|_| Err(UiError::internal()))
}

pub async fn run_with_vault_in_background<Output: Send + 'static>(
    shared_vault: &SharedVault,
    vault_task: impl FnOnce(&mut VaultService) -> UiResult<Output> + Send + 'static,
) -> UiResult<Output> {
    let shared_vault = shared_vault.clone();
    run_in_background(move || vault_task(&mut *shared_vault.lock_service()?)).await
}
