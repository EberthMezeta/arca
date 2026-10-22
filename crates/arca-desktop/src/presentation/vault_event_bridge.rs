//! Lleva al hilo de la interfaz los eventos que nacen en otros hilos (auto-bloqueo, D-Bus, portapapeles).
//! El sumidero existe antes que la ventana (lo necesita la raíz de composición); el manejador se instala después.

use std::cell::RefCell;
use std::sync::Arc;

use crate::vault_events::{VaultEvent, VaultEventSink};

type VaultEventHandler = Box<dyn Fn(VaultEvent)>;

thread_local! {
    static UI_THREAD_EVENT_HANDLER: RefCell<Option<VaultEventHandler>> = const { RefCell::new(None) };
}

/// Debe llamarse desde el hilo de la interfaz.
pub fn install_ui_thread_handler(vault_event_handler: impl Fn(VaultEvent) + 'static) {
    UI_THREAD_EVENT_HANDLER.with(|installed_handler| *installed_handler.borrow_mut() = Some(Box::new(vault_event_handler)));
}

/// Sin bucle de eventos (aún arrancando o ya cerrando) el evento se descarta: no hay nada que actualizar.
pub fn event_sink_to_ui_thread() -> VaultEventSink {
    Arc::new(|vault_event| {
        let _ = slint::invoke_from_event_loop(move || {
            UI_THREAD_EVENT_HANDLER.with(|installed_handler| {
                if let Some(vault_event_handler) = installed_handler.borrow().as_ref() {
                    vault_event_handler(vault_event);
                }
            });
        });
    })
}
