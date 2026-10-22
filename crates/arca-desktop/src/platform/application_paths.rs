use std::io;
use std::path::PathBuf;

/// Mismo identificador que usaba la versión Tauri: la bóveda sigue en `~/.local/share/dev.arca.desktop`.
pub const APPLICATION_IDENTIFIER: &str = "dev.arca.desktop";

pub fn application_data_directory() -> io::Result<PathBuf> {
    let data_root_directory =
        dirs::data_dir().ok_or_else(|| io::Error::other("el sistema no indica una carpeta de datos de usuario"))?;
    Ok(data_root_directory.join(APPLICATION_IDENTIFIER))
}
