use crate::domain::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::application::throttled_master_password_attempt::run_throttled;
use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_key_wrapping::unwrap_vault_key;
use crate::application::vault_session::{ActivityTimestamp, UnlockedVault};
use crate::domain::key_hierarchy::derive_vault_subkeys;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_file::vault_file_format::decode_vault_file;
use crate::domain::vault_revision::{VaultRollback, detect_rollback};

pub struct UnlockOutcome {
    pub unlocked_vault: UnlockedVault,
    /// `Some` si el archivo es más antiguo que el último que abrió o guardó esta app.
    pub detected_rollback: Option<VaultRollback>,
}

pub fn execute(
    vault_dependencies: &VaultDependencies,
    unlock_attempt_throttle: &mut UnlockAttemptThrottle,
    master_password: &SecretText,
) -> VaultResult<UnlockOutcome> {
    let unlocked_vault =
        run_throttled(vault_dependencies, unlock_attempt_throttle, || decrypt_vault(vault_dependencies, master_password))?;
    let opened_revision = unlocked_vault.vault_contents.revision();
    let detected_rollback =
        detect_rollback(vault_dependencies.vault_revision_store.load_last_seen_revision(), opened_revision);
    // Se avisa una vez: a partir de aquí la versión abierta es la de referencia.
    if let Some(opened_revision) = opened_revision {
        let _ = vault_dependencies.vault_revision_store.record_revision(&opened_revision);
    }
    Ok(UnlockOutcome { unlocked_vault, detected_rollback })
}

fn decrypt_vault(vault_dependencies: &VaultDependencies, master_password: &SecretText) -> VaultResult<UnlockedVault> {
    let vault_file_bytes = vault_dependencies.vault_repository.read_vault_bytes()?;
    let decoded_vault_file = decode_vault_file(&vault_file_bytes)?;
    let vault_header = decoded_vault_file.vault_header;
    if !vault_dependencies.key_derivation_policy.accepts(&vault_header.key_derivation_parameters) {
        return Err(VaultError::InsecureKeyDerivationParameters);
    }

    let vault_key = unwrap_vault_key(vault_dependencies, master_password, &vault_header)?;
    let vault_subkeys = derive_vault_subkeys(&vault_key)?;
    let serialized_contents = vault_dependencies
        .authenticated_cipher
        .open(
            &vault_subkeys.vault_contents_key,
            &decoded_vault_file.encrypted_contents,
            &decoded_vault_file.encoded_header_bytes,
        )
        .map_err(|_| VaultError::InvalidMasterPasswordOrCorruptedVault)?;
    let vault_contents: VaultContents =
        serde_json::from_slice(&serialized_contents).map_err(VaultError::corrupted_file)?;

    Ok(UnlockedVault {
        vault_key,
        vault_header,
        encoded_header_bytes: decoded_vault_file.encoded_header_bytes,
        vault_subkeys,
        vault_contents,
        last_activity: ActivityTimestamp::now(vault_dependencies.clock.as_ref()),
    })
}
