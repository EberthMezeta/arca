//! Elementos y carpetas: consultas (siempre enmascaradas) y cambios.


use uuid::Uuid;

use crate::application::analyze_password_health::{self, PasswordHealthReport};
use crate::application::manage_attachments::{self};
use crate::application::save_vault_item::{self, SaveVaultItemRequest};
use crate::application::search_vault_items::{self, VaultItemFilter, VaultItemSummary, VaultOverview};
use crate::application::{
    manage_folders, manage_item_lifecycle,
};
use crate::domain::folder::Folder;
use crate::domain::secret_text::SecretText;
use crate::domain::totp_configuration::{TotpCode, TotpConfiguration};
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_item::{RevealableField, VaultItem};

use super::VaultService;

pub trait VaultItems {
    fn vault_overview(&self) -> VaultResult<VaultOverview>;

    fn search_items(&self, item_filter: &VaultItemFilter) -> VaultResult<Vec<VaultItemSummary>>;

    /// Elemento con los secretos enmascarados, tanto para la vista de detalle como para el editor.
    /// Los secretos solo cruzan a la UI con `reveal_item_field`, y al guardar los marcadores
    /// sin tocar recuperan su valor (ver `save_vault_item`).
    fn get_item_details(&self, item_id: Uuid) -> VaultResult<VaultItem>;

    fn list_folders(&self) -> VaultResult<Vec<Folder>>;

    fn reveal_item_field(&self, item_id: Uuid, requested_field: RevealableField) -> VaultResult<SecretText>;

    /// No cuenta como actividad: la UI lo consulta periódicamente mientras muestra el código.
    fn current_totp_code(&self, item_id: Uuid) -> VaultResult<TotpCode>;

    fn password_health_report(&self) -> VaultResult<PasswordHealthReport>;

    fn save_item(&mut self, save_request: SaveVaultItemRequest) -> VaultResult<VaultItem>;

    fn move_item_to_trash(&mut self, item_id: Uuid) -> VaultResult<()>;

    fn restore_item_from_trash(&mut self, item_id: Uuid) -> VaultResult<()>;

    fn delete_item_permanently(&mut self, item_id: Uuid) -> VaultResult<()>;

    fn empty_trash(&mut self) -> VaultResult<usize>;

    fn create_folder(&mut self, folder_name: &str) -> VaultResult<Folder>;

    fn rename_folder(&mut self, folder_id: Uuid, folder_name: &str) -> VaultResult<Folder>;

    fn delete_folder(&mut self, folder_id: Uuid) -> VaultResult<()>;
}

impl VaultItems for VaultService {
    fn vault_overview(&self) -> VaultResult<VaultOverview> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(search_vault_items::build_overview(&unlocked_vault.vault_contents))
    }

    fn search_items(&self, item_filter: &VaultItemFilter) -> VaultResult<Vec<VaultItemSummary>> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(search_vault_items::search_items(&unlocked_vault.vault_contents, item_filter))
    }

    fn get_item_details(&self, item_id: Uuid) -> VaultResult<VaultItem> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(unlocked_vault.vault_contents.find_item(item_id)?.redacted_copy())
    }

    fn list_folders(&self) -> VaultResult<Vec<Folder>> {
        Ok(self.session_state.require_unlocked()?.vault_contents.folders.clone())
    }

    fn reveal_item_field(&self, item_id: Uuid, requested_field: RevealableField) -> VaultResult<SecretText> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        unlocked_vault.vault_contents.find_item(item_id)?.reveal_field(requested_field)
    }

    fn current_totp_code(&self, item_id: Uuid) -> VaultResult<TotpCode> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        let vault_item = unlocked_vault.vault_contents.find_item(item_id)?;
        let totp_secret = vault_item.item_content.totp_secret().ok_or(VaultError::FieldNotAvailable)?;
        let unix_timestamp_seconds = u64::try_from(self.now().timestamp()).map_err(|_| VaultError::InvalidSystemClock)?;
        TotpConfiguration::parse(totp_secret)?.code_at(unix_timestamp_seconds)
    }

    fn password_health_report(&self) -> VaultResult<PasswordHealthReport> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(analyze_password_health::execute(&unlocked_vault.vault_contents, self.now()))
    }

    fn save_item(&mut self, save_request: SaveVaultItemRequest) -> VaultResult<VaultItem> {
        let now = self.now();
        self.mutate(|vault_contents| {
            save_vault_item::execute(vault_contents, save_request, now)
        })
    }

    fn move_item_to_trash(&mut self, item_id: Uuid) -> VaultResult<()> {
        let now = self.now();
        self.mutate(|vault_contents| {
            manage_item_lifecycle::move_to_trash(vault_contents, item_id, now)
        })
    }

    fn restore_item_from_trash(&mut self, item_id: Uuid) -> VaultResult<()> {
        let now = self.now();
        self.mutate(|vault_contents| {
            manage_item_lifecycle::restore_from_trash(vault_contents, item_id, now)
        })
    }

    fn delete_item_permanently(&mut self, item_id: Uuid) -> VaultResult<()> {
        let removed_items = self.mutate_discarding_backups(|vault_contents| {
            manage_item_lifecycle::delete_permanently(vault_contents, item_id)
        })?;
        manage_attachments::delete_attachments_of_items(&self.vault_dependencies, &removed_items);
        Ok(())
    }

    fn empty_trash(&mut self) -> VaultResult<usize> {
        let removed_items = self.mutate_discarding_backups(|vault_contents| {
            Ok(manage_item_lifecycle::empty_trash(vault_contents))
        })?;
        manage_attachments::delete_attachments_of_items(&self.vault_dependencies, &removed_items);
        Ok(removed_items.len())
    }

    fn create_folder(&mut self, folder_name: &str) -> VaultResult<Folder> {
        self.mutate(|vault_contents| {
            manage_folders::create_folder(vault_contents, folder_name)
        })
    }

    fn rename_folder(&mut self, folder_id: Uuid, folder_name: &str) -> VaultResult<Folder> {
        self.mutate(|vault_contents| {
            manage_folders::rename_folder(vault_contents, folder_id, folder_name)
        })
    }

    fn delete_folder(&mut self, folder_id: Uuid) -> VaultResult<()> {
        self.mutate(|vault_contents| {
            manage_folders::delete_folder(vault_contents, folder_id)
        })
    }
}
