use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use uuid::Uuid;

use crate::domain::ports::attachment_storage::AttachmentStorage;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::infrastructure::storage::owner_only_files::{create_owner_only_directory, write_owner_only_file};

/// Un archivo `<uuid>.bin` por adjunto cifrado; el nombre no revela nada del contenido.
pub struct DirectoryAttachmentStorage {
    attachments_directory: PathBuf,
}

impl DirectoryAttachmentStorage {
    pub fn new(attachments_directory: impl Into<PathBuf>) -> Self {
        Self { attachments_directory: attachments_directory.into() }
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
        fs::read(self.attachment_file_path(attachment_id)).map_err(|read_error| match read_error.kind() {
            ErrorKind::NotFound => VaultError::AttachmentNotFound { attachment_id },
            _ => VaultError::storage(read_error),
        })
    }

    fn delete_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<()> {
        match fs::remove_file(self.attachment_file_path(attachment_id)) {
            Ok(()) => Ok(()),
            Err(delete_error) if delete_error.kind() == ErrorKind::NotFound => Ok(()),
            Err(delete_error) => Err(VaultError::storage(delete_error)),
        }
    }
}
