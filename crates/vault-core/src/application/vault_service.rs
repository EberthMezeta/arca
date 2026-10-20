//! Fachada: único punto de entrada para la capa de presentación (comandos Tauri).
//! Cada método delega en un caso de uso; aquí solo se gestiona el estado de sesión.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::application::analyze_password_health::{self, PasswordHealthReport};
use crate::application::import_export_vault::{self, ExportedFile, ImportSummary};
use crate::application::manage_attachments::{self, DecryptedAttachment};
use crate::application::save_vault_item::{self, SaveVaultItemRequest};
use crate::application::search_vault_items::{self, VaultItemFilter, VaultItemSummary, VaultOverview};
use crate::application::throttled_master_password_attempt::run_throttled;
use crate::domain::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_key_wrapping::unwrap_vault_key;
use crate::application::vault_session::{UnlockedVault, VaultSessionState};
use crate::application::{
    change_master_password, create_vault, manage_folders, manage_item_lifecycle, unlock_vault,
};
use crate::domain::folder::Folder;
use crate::domain::password_generation::password_generation_options::PasswordGenerationOptions;
use crate::domain::password_strength::PasswordStrength;
use crate::domain::ports::clock::Clock;
use crate::domain::ports::vault_exporter::ExportFormat;
use crate::domain::ports::vault_importer::ImportFormat;
use crate::domain::secret_text::SecretText;
use crate::domain::totp_configuration::{TotpCode, TotpConfiguration};
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_item::{AttachmentReference, RevealableField, VaultItem};
use crate::domain::vault_preferences::VaultPreferences;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub vault_exists: bool,
    pub is_unlocked: bool,
    pub unlock_retry_after_seconds: Option<u64>,
}

pub struct VaultService {
    vault_dependencies: VaultDependencies,
    session_state: VaultSessionState,
    unlock_attempt_throttle: UnlockAttemptThrottle,
}

impl VaultService {
    pub fn new(vault_dependencies: VaultDependencies) -> Self {
        let unlock_attempt_throttle = vault_dependencies.unlock_throttle_store.load_throttle().unwrap_or_default();
        Self { vault_dependencies, session_state: VaultSessionState::Locked, unlock_attempt_throttle }
    }

    /// Descarta claves y secretos de memoria y vacía el portapapeles si aún tiene un secreto nuestro.
    fn lock_session(&mut self) {
        self.session_state.lock();
        self.vault_dependencies.secret_clipboard.clear_copied_secret();
    }

    /// Acceso de escritura: además cuenta como actividad del usuario (retrasa el auto-bloqueo).
    fn unlocked_vault_for_change<'session>(
        session_state: &'session mut VaultSessionState,
        clock: &dyn Clock,
    ) -> VaultResult<&'session mut UnlockedVault> {
        let unlocked_vault = session_state.require_unlocked_mut()?;
        unlocked_vault.last_activity_at = clock.now();
        Ok(unlocked_vault)
    }

    fn now(&self) -> DateTime<Utc> {
        self.vault_dependencies.clock.now()
    }

    // ── Sesión ────────────────────────────────────────────────────────────────

    pub fn vault_status(&self) -> VaultStatus {
        VaultStatus {
            vault_exists: self.vault_dependencies.vault_repository.vault_exists(),
            is_unlocked: self.session_state.is_unlocked(),
            unlock_retry_after_seconds: self.unlock_attempt_throttle.seconds_until_next_attempt(self.now()),
        }
    }

    pub fn create_vault(&mut self, master_password: &SecretText) -> VaultResult<()> {
        let unlocked_vault = create_vault::execute(&self.vault_dependencies, master_password)?;
        self.session_state = VaultSessionState::Unlocked(Box::new(unlocked_vault));
        Ok(())
    }

    pub fn unlock_vault(&mut self, master_password: &SecretText) -> VaultResult<()> {
        let unlocked_vault =
            unlock_vault::execute(&self.vault_dependencies, &mut self.unlock_attempt_throttle, master_password)?;
        self.session_state = VaultSessionState::Unlocked(Box::new(unlocked_vault));
        Ok(())
    }

    pub fn lock_vault(&mut self) {
        self.lock_session();
    }

    pub fn register_user_activity(&mut self) -> VaultResult<()> {
        Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref()).map(|_| ())
    }

    /// Lo llama periódicamente el planificador de auto-bloqueo. Devuelve `true` si ha bloqueado.
    pub fn lock_if_idle(&mut self) -> bool {
        let now = self.now();
        let Ok(unlocked_vault) = self.session_state.require_unlocked() else {
            return false;
        };
        let idle_seconds = (now - unlocked_vault.last_activity_at).num_seconds();
        let auto_lock_after_seconds = unlocked_vault.vault_contents.preferences.auto_lock_after_idle_seconds;
        let is_idle_too_long = u64::try_from(idle_seconds).is_ok_and(|idle_seconds| idle_seconds >= auto_lock_after_seconds);
        if is_idle_too_long {
            self.lock_session();
        }
        is_idle_too_long
    }

    pub fn change_master_password(
        &mut self,
        current_master_password: &SecretText,
        new_master_password: &SecretText,
    ) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        run_throttled(&self.vault_dependencies, &mut self.unlock_attempt_throttle, || {
            change_master_password::execute(
                &self.vault_dependencies,
                unlocked_vault,
                current_master_password,
                new_master_password,
            )
        })
    }

    // ── Consultas ─────────────────────────────────────────────────────────────

    pub fn vault_overview(&self) -> VaultResult<VaultOverview> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(search_vault_items::build_overview(&unlocked_vault.vault_contents))
    }

    pub fn search_items(&self, item_filter: &VaultItemFilter) -> VaultResult<Vec<VaultItemSummary>> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(search_vault_items::search_items(&unlocked_vault.vault_contents, item_filter))
    }

    /// Elemento con los secretos enmascarados, tanto para la vista de detalle como para el editor.
    /// Los secretos solo cruzan a la UI con `reveal_item_field`, y al guardar los marcadores
    /// sin tocar recuperan su valor (ver `save_vault_item`).
    pub fn get_item_details(&self, item_id: Uuid) -> VaultResult<VaultItem> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(unlocked_vault.vault_contents.find_item(item_id)?.redacted_copy())
    }

    pub fn list_folders(&self) -> VaultResult<Vec<Folder>> {
        Ok(self.session_state.require_unlocked()?.vault_contents.folders.clone())
    }

    pub fn reveal_item_field(&self, item_id: Uuid, requested_field: RevealableField) -> VaultResult<SecretText> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        unlocked_vault.vault_contents.find_item(item_id)?.reveal_field(requested_field)
    }

    /// No cuenta como actividad: la UI lo consulta periódicamente mientras muestra el código.
    pub fn current_totp_code(&self, item_id: Uuid) -> VaultResult<TotpCode> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        let vault_item = unlocked_vault.vault_contents.find_item(item_id)?;
        let totp_secret = vault_item.item_content.totp_secret().ok_or(VaultError::FieldNotAvailable)?;
        let unix_timestamp_seconds = u64::try_from(self.now().timestamp()).map_err(|_| VaultError::Cryptography)?;
        TotpConfiguration::parse(totp_secret)?.code_at(unix_timestamp_seconds)
    }

    pub fn password_health_report(&self) -> VaultResult<PasswordHealthReport> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        Ok(analyze_password_health::execute(&unlocked_vault.vault_contents, self.now()))
    }

    pub fn preferences(&self) -> VaultResult<VaultPreferences> {
        Ok(self.session_state.require_unlocked()?.vault_contents.preferences.clone())
    }

    // ── Portapapeles ──────────────────────────────────────────────────────────

    pub fn copy_item_field_to_clipboard(&mut self, item_id: Uuid, requested_field: RevealableField) -> VaultResult<()> {
        let secret_value = self.reveal_item_field(item_id, requested_field)?;
        self.copy_secret_to_clipboard(secret_value)
    }

    pub fn copy_totp_code_to_clipboard(&mut self, item_id: Uuid) -> VaultResult<()> {
        let totp_code = self.current_totp_code(item_id)?;
        self.copy_secret_to_clipboard(SecretText::new(totp_code.current_code))
    }

    pub fn copy_secret_to_clipboard(&mut self, secret_value: SecretText) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        let clear_after = Duration::from_secs(unlocked_vault.vault_contents.preferences.clipboard_clear_after_seconds);
        self.vault_dependencies.secret_clipboard.copy_with_automatic_clear(secret_value, clear_after)
    }

    // ── Elementos y carpetas ──────────────────────────────────────────────────

    pub fn save_item(&mut self, save_request: SaveVaultItemRequest) -> VaultResult<VaultItem> {
        let now = self.now();
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            save_vault_item::execute(vault_contents, save_request, now)
        })
    }

    pub fn move_item_to_trash(&mut self, item_id: Uuid) -> VaultResult<()> {
        let now = self.now();
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            manage_item_lifecycle::move_to_trash(vault_contents, item_id, now)
        })
    }

    pub fn restore_item_from_trash(&mut self, item_id: Uuid) -> VaultResult<()> {
        let now = self.now();
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            manage_item_lifecycle::restore_from_trash(vault_contents, item_id, now)
        })
    }

    pub fn delete_item_permanently(&mut self, item_id: Uuid) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        let removed_items = unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            manage_item_lifecycle::delete_permanently(vault_contents, item_id)
        })?;
        manage_attachments::delete_attachments_of_items(&self.vault_dependencies, &removed_items);
        Ok(())
    }

    pub fn empty_trash(&mut self) -> VaultResult<usize> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        let removed_items = unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            Ok(manage_item_lifecycle::empty_trash(vault_contents))
        })?;
        manage_attachments::delete_attachments_of_items(&self.vault_dependencies, &removed_items);
        Ok(removed_items.len())
    }

    pub fn create_folder(&mut self, folder_name: &str) -> VaultResult<Folder> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            manage_folders::create_folder(vault_contents, folder_name)
        })
    }

    pub fn rename_folder(&mut self, folder_id: Uuid, folder_name: &str) -> VaultResult<Folder> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            manage_folders::rename_folder(vault_contents, folder_id, folder_name)
        })
    }

    pub fn delete_folder(&mut self, folder_id: Uuid) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            manage_folders::delete_folder(vault_contents, folder_id)
        })
    }

    pub fn update_preferences(&mut self, new_preferences: VaultPreferences) -> VaultResult<VaultPreferences> {
        new_preferences.validate()?;
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            vault_contents.preferences = new_preferences.clone();
            Ok(new_preferences)
        })
    }

    // ── Adjuntos ──────────────────────────────────────────────────────────────

    pub fn add_attachment(&mut self, item_id: Uuid, file_name: &str, file_bytes: &[u8]) -> VaultResult<AttachmentReference> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        manage_attachments::add_attachment(&self.vault_dependencies, unlocked_vault, item_id, file_name, file_bytes)
    }

    pub fn read_attachment(&self, item_id: Uuid, attachment_id: Uuid) -> VaultResult<DecryptedAttachment> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        manage_attachments::read_attachment(&self.vault_dependencies, unlocked_vault, item_id, attachment_id)
    }

    pub fn delete_attachment(&mut self, item_id: Uuid, attachment_id: Uuid) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        manage_attachments::delete_attachment(&self.vault_dependencies, unlocked_vault, item_id, attachment_id)
    }

    // ── Importar / exportar ───────────────────────────────────────────────────

    pub fn import_file(
        &mut self,
        import_format: ImportFormat,
        file_bytes: &[u8],
        file_password: Option<&SecretText>,
    ) -> VaultResult<ImportSummary> {
        let imported_vault_data =
            import_export_vault::parse_import_file(&self.vault_dependencies, import_format, file_bytes, file_password)?;
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, |vault_contents| {
            Ok(import_export_vault::merge_imported_data(vault_contents, imported_vault_data))
        })
    }

    /// Exportar saca toda la bóveda del cifrado: exige la contraseña maestra (con retardo),
    /// para que alguien con acceso breve a la app desbloqueada no pueda llevársela.
    pub fn export_vault(
        &mut self,
        master_password: &SecretText,
        export_format: ExportFormat,
        file_password: Option<&SecretText>,
    ) -> VaultResult<ExportedFile> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        run_throttled(&self.vault_dependencies, &mut self.unlock_attempt_throttle, || {
            unwrap_vault_key(&self.vault_dependencies, master_password, &unlocked_vault.vault_header)
        })?;
        import_export_vault::export_vault(&self.vault_dependencies, &unlocked_vault.vault_contents, export_format, file_password)
    }

    // ── Utilidades sin sesión ─────────────────────────────────────────────────

    pub fn generate_password(&self, generation_options: PasswordGenerationOptions) -> VaultResult<SecretText> {
        generation_options.into_strategy().generate_password()
    }

    pub fn estimate_password_strength(&self, candidate_password: &SecretText) -> PasswordStrength {
        PasswordStrength::estimate(candidate_password)
    }
}
