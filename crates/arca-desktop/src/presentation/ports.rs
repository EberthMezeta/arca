//! Puertos de la capa de presentación: lo que los controladores necesitan del sistema (elegir y
//! escribir archivos) sin saber cómo se hace. Las implementaciones reales están en `platform`;
//! los tests usan dobles.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use zeroize::Zeroizing;

use crate::ui_error::{UiError, UiResult};

/// Ruta elegida por el usuario, o `None` si canceló.
pub type ChosenPath = Pin<Box<dyn Future<Output = Option<PathBuf>>>>;

/// Diálogos de archivo. Asíncronos: la ventana sigue dibujándose mientras están abiertos.
pub trait FileDialogs {
    fn choose_file_to_open(&self, dialog_title: &str) -> ChosenPath;
    fn choose_file_to_save(&self, dialog_title: &str, suggested_file_name: &str) -> ChosenPath;
}

/// Archivos que elige el usuario (importaciones, adjuntos, exportaciones). Se usa desde hilos en
/// segundo plano, por eso es `Send + Sync`.
pub trait UserFiles: Send + Sync {
    /// Lee como mucho `maximum_byte_count` bytes; si el archivo es mayor devuelve `too_large_error`.
    fn read_limited(
        &self,
        file_path: &Path,
        maximum_byte_count: u64,
        too_large_error: UiError,
    ) -> UiResult<Zeroizing<Vec<u8>>>;

    /// Escribe con permisos solo para el usuario (puede ir en claro: exportaciones, adjuntos).
    fn write_private(&self, file_path: &Path, file_bytes: &[u8]) -> UiResult<()>;
}
