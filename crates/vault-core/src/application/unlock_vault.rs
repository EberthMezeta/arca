use crate::application::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_key_wrapping::unwrap_vault_key;
use crate::application::vault_session::UnlockedVault;
use crate::domain::key_hierarchy::derive_vault_subkeys;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_file::vault_file_format::decode_vault_file;

pub fn execute(
    vault_dependencies: &VaultDependencies,
    unlock_attempt_throttle: &mut UnlockAttemptThrottle,
    master_password: &SecretText,
) -> VaultResult<UnlockedVault> {
    let now = vault_dependencies.clock.now();
    if let Some(retry_after_seconds) = unlock_attempt_throttle.seconds_until_next_attempt(now) {
        return Err(VaultError::UnlockTemporarilyBlocked { retry_after_seconds });
    }

    let unlock_result = decrypt_vault(vault_dependencies, master_password);
    match &unlock_result {
        Ok(_) => unlock_attempt_throttle.register_successful_attempt(),
        Err(VaultError::InvalidMasterPasswordOrCorruptedVault) => unlock_attempt_throttle.register_failed_attempt(now),
        Err(_) => {}
    }
    unlock_result
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
        last_activity_at: vault_dependencies.clock.now(),
    })
}
