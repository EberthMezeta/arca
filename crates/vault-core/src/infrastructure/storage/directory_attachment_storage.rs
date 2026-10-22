use std::io::ErrorKind;
use std::path::PathBuf;

use uuid::Uuid;

use crate::domain::ports::attachment_storage::AttachmentStorage;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::infrastructure::storage::bounded_file_reader::{BoundedReadError, read_file_with_size_limit};
use crate::infrastructure::storage::owner_only_files::{create_owner_only_directory, remove_file_if_exists, write_owner_only_file};

/// Un archivo `<uuid>.bin` por adjunto cifrado; el nombre no revela nada del contenido.
/// Lo que el cifrado añade a cada adjunto (longitud y nonce, etiqueta), con margen.
const SEALED_ATTACHMENT_OVERHEAD_BYTES: u64 = 64;

pub struct DirectoryAttachmentStorage {
    attachments_directory: PathBuf,
    /// Un blob mayor que el mayor adjunto permitido lo ha puesto alguien con acceso al disco.
    maximum_encrypted_size_bytes: u64,
}

impl DirectoryAttachmentStorage {
    pub fn new(attachments_directory: impl Into<PathBuf>, maximum_attachment_size_bytes: u64) -> Self {
        Self {
            attachments_directory: attachments_directory.into(),
            maximum_encrypted_size_bytes: maximum_attachment_size_bytes.saturating_add(SEALED_ATTACHMENT_OVERHEAD_BYTES),
        }
    }

    fn attachment_file_path(&self, attachment_id: Uuid) -> PathBuf {
        self.attachments_directory.join(format!("{attachment_id}.bin"))
    }
}

impl AttachmentStorage for DirectoryAttachmentStorage {
    fn write_encrypted_attachment(&self, attachment_id: Uuid, encrypted_bytes: &[u8]) -> VaultResult<()> {
        create_owner_only_directory(&self.attachments_directory)?;
        write_owner_only_file(&self.attachment_file_path(attachment_id), encrypted_bytes)
    }

    fn read_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<Vec<u8>> {
        read_file_with_size_limit(&self.attachment_file_path(attachment_id), self.maximum_encrypted_size_bytes)
            .map(|mut encrypted_bytes| std::mem::take(&mut *encrypted_bytes))
            .map_err(|read_error| match read_error {
                BoundedReadError::TooLarge => VaultError::corrupted_file("adjunto dañado"),
                BoundedReadError::Unreadable(io_error) if io_error.kind() == ErrorKind::NotFound => {
                    VaultError::AttachmentNotFound { attachment_id }
                }
                BoundedReadError::Unreadable(io_error) => VaultError::storage(io_error),
            })
    }

    fn delete_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<()> {
        remove_file_if_exists(&self.attachment_file_path(attachment_id))
    }
}
