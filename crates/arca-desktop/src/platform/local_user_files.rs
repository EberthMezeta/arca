//! Archivos del usuario en el disco local: lectura acotada y escritura con permisos `0600`.

use std::path::Path;

use vault_core::infrastructure::storage::bounded_file_reader::{BoundedReadError, read_file_with_size_limit};
use vault_core::infrastructure::storage::owner_only_files::write_owner_only_file;
use zeroize::Zeroizing;

use crate::presentation::ports::UserFiles;
use crate::ui_error::{UiError, UiResult};

pub struct LocalUserFiles;

impl UserFiles for LocalUserFiles {
    fn read_limited(
        &self,
        file_path: &Path,
        maximum_byte_count: u64,
        too_large_error: UiError,
    ) -> UiResult<Zeroizing<Vec<u8>>> {
        read_file_with_size_limit(file_path, maximum_byte_count).map_err(|read_error| match read_error {
            BoundedReadError::TooLarge => too_large_error,
            BoundedReadError::Unreadable(io_error) => UiError::file_access(io_error),
        })
    }

    fn write_private(&self, file_path: &Path, file_bytes: &[u8]) -> UiResult<()> {
        write_owner_only_file(file_path, file_bytes).map_err(UiError::file_write)
    }
}
