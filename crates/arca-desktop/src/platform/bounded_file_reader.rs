//! Lectura acotada de archivos elegidos por el usuario, con el error que se le muestra.

use std::path::Path;

use vault_core::infrastructure::storage::bounded_file_reader::{BoundedReadError, read_file_with_size_limit};
use zeroize::Zeroizing;

use crate::ui_error::{UiError, UiResult};

/// `too_large_error` es el error que se muestra si el archivo pasa de `maximum_byte_count`.
pub fn read_user_file(file_path: &Path, maximum_byte_count: u64, too_large_error: UiError) -> UiResult<Zeroizing<Vec<u8>>> {
    read_file_with_size_limit(file_path, maximum_byte_count).map_err(|read_error| match read_error {
        BoundedReadError::TooLarge => too_large_error,
        BoundedReadError::Unreadable(io_error) => UiError::file_access(io_error),
    })
}
