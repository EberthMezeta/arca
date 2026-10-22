//! Adjuntos cifrados de un elemento.


use uuid::Uuid;

use crate::application::manage_attachments::{self, DecryptedAttachment};
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;
use crate::domain::vault_item::AttachmentReference;

use super::VaultService;

pub trait VaultAttachments {
    fn add_attachment(&mut self, item_id: Uuid, file_name: &str, file_bytes: &[u8]) -> VaultResult<AttachmentReference>;

    /// Guardar un adjunto descifrado lo saca del cifrado, como exportar: exige la contraseña
    /// maestra (con retardo), para que alguien con acceso breve a la app no se lleve los archivos.
    fn read_attachment(
        &mut self,
        master_password: &SecretText,
        item_id: Uuid,
        attachment_id: Uuid,
    ) -> VaultResult<DecryptedAttachment>;

    fn delete_attachment(&mut self, item_id: Uuid, attachment_id: Uuid) -> VaultResult<()>;
}

impl VaultAttachments for VaultService {
    fn add_attachment(&mut self, item_id: Uuid, file_name: &str, file_bytes: &[u8]) -> VaultResult<AttachmentReference> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        manage_attachments::add_attachment(&self.vault_dependencies, unlocked_vault, item_id, file_name, file_bytes)
    }

    fn read_attachment(
        &mut self,
        master_password: &SecretText,
        item_id: Uuid,
        attachment_id: Uuid,
    ) -> VaultResult<DecryptedAttachment> {
        self.reauthenticate(master_password)?;
        let unlocked_vault = self.session_state.require_unlocked()?;
        manage_attachments::read_attachment(&self.vault_dependencies, unlocked_vault, item_id, attachment_id)
    }

    fn delete_attachment(&mut self, item_id: Uuid, attachment_id: Uuid) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        manage_attachments::delete_attachment(&self.vault_dependencies, unlocked_vault, item_id, attachment_id)
    }
}
