//! Copiar secretos al portapapeles (se vacía solo).

use std::time::Duration;

use uuid::Uuid;

use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;
use crate::domain::vault_item::RevealableField;

use super::{VaultItems, VaultService};

pub trait VaultClipboard {
    fn copy_item_field_to_clipboard(&mut self, item_id: Uuid, requested_field: RevealableField) -> VaultResult<()>;

    fn copy_totp_code_to_clipboard(&mut self, item_id: Uuid) -> VaultResult<()>;

    fn copy_secret_to_clipboard(&mut self, secret_value: SecretText) -> VaultResult<()>;
}

impl VaultClipboard for VaultService {
    fn copy_item_field_to_clipboard(&mut self, item_id: Uuid, requested_field: RevealableField) -> VaultResult<()> {
        let secret_value = self.reveal_item_field(item_id, requested_field)?;
        self.copy_secret_to_clipboard(secret_value)
    }

    fn copy_totp_code_to_clipboard(&mut self, item_id: Uuid) -> VaultResult<()> {
        let totp_code = self.current_totp_code(item_id)?;
        self.copy_secret_to_clipboard(SecretText::new(totp_code.current_code))
    }

    fn copy_secret_to_clipboard(&mut self, secret_value: SecretText) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        let clear_after = Duration::from_secs(unlocked_vault.vault_contents.preferences.clipboard_clear_after_seconds);
        self.vault_dependencies.secret_clipboard.copy_with_automatic_clear(secret_value, clear_after)
    }
}
