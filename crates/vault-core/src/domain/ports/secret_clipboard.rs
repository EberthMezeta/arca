use std::time::Duration;

use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;

/// Portapapeles que se limpia solo tras `clear_after` si nadie lo ha cambiado.
pub trait SecretClipboard: Send + Sync {
    fn copy_with_automatic_clear(&self, secret_value: SecretText, clear_after: Duration) -> VaultResult<()>;
}
