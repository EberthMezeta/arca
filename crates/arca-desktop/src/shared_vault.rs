//! `VaultService` compartido entre la interfaz y los hilos en segundo plano
//! (Argon2id, auto-bloqueo, señales del sistema).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};

use vault_core::application::vault_service::VaultService;

use crate::ui_error::{UiError, UiResult};
use crate::vault_events::{VaultEvent, VaultEventSink, VaultLockReason};

#[derive(Clone)]
pub struct SharedVault {
    vault_service: Arc<Mutex<VaultService>>,
}

impl SharedVault {
    pub fn new(vault_service: VaultService) -> Self {
        Self { vault_service: Arc::new(Mutex::new(vault_service)) }
    }

    pub fn lock_service(&self) -> UiResult<MutexGuard<'_, VaultService>> {
        self.vault_service.lock().map_err(|_| UiError::internal())
    }

    /// Para tareas accesorias desde la interfaz (actividad, comprobación de minimizado):
    /// si un hilo está derivando la clave con Argon2id, se omiten en vez de congelar la ventana.
    pub fn try_lock_service(&self) -> Option<MutexGuard<'_, VaultService>> {
        self.vault_service.try_lock().ok()
    }

    /// Para bloquear la bóveda hay que poder entrar aunque el mutex esté envenenado (un pánico
    /// con él tomado): si no, las claves seguirían en memoria y ni el auto-bloqueo funcionaría.
    fn lock_service_even_if_poisoned(&self) -> MutexGuard<'_, VaultService> {
        self.vault_service.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Bloquea ya (suspensión, bloqueo de pantalla, cierre). Devuelve `true` si estaba desbloqueada.
    pub fn lock_if_unlocked(&self) -> bool {
        let mut vault_service = self.lock_service_even_if_poisoned();
        let was_unlocked = vault_service.vault_status().is_unlocked;
        vault_service.lock_vault();
        was_unlocked
    }

    pub fn lock_if_idle(&self) -> bool {
        self.lock_service_even_if_poisoned().lock_if_idle()
    }

    /// Bloquea si el usuario activó "bloquear al minimizar". Devuelve `true` si bloqueó.
    /// Si Argon2id tiene el mutex se omite (la ventana se vuelve a comprobar enseguida),
    /// pero un mutex envenenado no impide bloquear.
    pub fn lock_if_minimize_policy_applies(&self) -> bool {
        let mut vault_service = match self.vault_service.try_lock() {
            Ok(vault_service) => vault_service,
            Err(TryLockError::Poisoned(poisoned_guard)) => poisoned_guard.into_inner(),
            Err(TryLockError::WouldBlock) => return false,
        };
        let should_lock = vault_service.vault_status().is_unlocked
            && vault_service
                .preferences()
                .is_ok_and(|vault_preferences| vault_preferences.should_lock_on_window_minimize);
        if should_lock {
            vault_service.lock_vault();
        }
        should_lock
    }
}

/// Bloquea la bóveda y avisa a la interfaz del motivo. Se clona a cada hilo que puede bloquear.
#[derive(Clone)]
pub struct VaultLocker {
    shared_vault: SharedVault,
    vault_event_sink: VaultEventSink,
}

impl VaultLocker {
    pub fn new(shared_vault: SharedVault, vault_event_sink: VaultEventSink) -> Self {
        Self { shared_vault, vault_event_sink }
    }

    pub fn lock_and_notify(&self, lock_reason: VaultLockReason) {
        if self.shared_vault.lock_if_unlocked() {
            (self.vault_event_sink)(VaultEvent::VaultLocked(lock_reason));
        }
    }

    pub fn lock_if_idle_and_notify(&self) {
        if self.shared_vault.lock_if_idle() {
            (self.vault_event_sink)(VaultEvent::VaultLocked(VaultLockReason::IdleTimeout));
        }
    }
}
