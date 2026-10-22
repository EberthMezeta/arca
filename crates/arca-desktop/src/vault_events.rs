//! Eventos que nacen fuera del hilo de la interfaz (temporizadores, D-Bus, portapapeles)
//! y que la interfaz debe reflejar. Patrón Observer: los productores solo conocen el sumidero.

use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultLockReason {
    IdleTimeout,
    WindowMinimized,
    ScreenLocked,
    SystemSuspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultEvent {
    VaultLocked(VaultLockReason),
    ClipboardCleared,
}

/// Se puede llamar desde cualquier hilo.
pub type VaultEventSink = Arc<dyn Fn(VaultEvent) + Send + Sync>;
