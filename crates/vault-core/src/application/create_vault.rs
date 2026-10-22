use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_key_wrapping::build_header_wrapping_vault_key;
use crate::application::vault_session::{UnlockedVault, persist_vault_contents};
use crate::domain::key_hierarchy::derive_vault_subkeys;
use crate::domain::master_password_policy::MasterPasswordPolicy;
use crate::domain::secret_text::SecretText;
use crate::domain::secure_random::random_encryption_key;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};

/// Crea una bóveda vacía y la deja desbloqueada.
pub fn execute(vault_dependencies: &VaultDependencies, master_password: &SecretText) -> VaultResult<UnlockedVault> {
    if vault_dependencies.vault_repository.vault_exists() {
        return Err(VaultError::VaultAlreadyExists);
    }
    let violations = MasterPasswordPolicy::default().find_violations(master_password);
    if !violations.is_empty() {
        return Err(VaultError::WeakMasterPassword { violations });
    }

    let vault_key = random_encryption_key()?;
    let vault_header = build_header_wrapping_vault_key(
        vault_dependencies,
        master_password,
        &vault_key,
        vault_dependencies.key_derivation_policy.parameters_for_new_vaults,
    )?;
    let encoded_header_bytes = vault_header.encode()?;
    let vault_subkeys = derive_vault_subkeys(&vault_key)?;
    let mut vault_contents = VaultContents::default();

    persist_vault_contents(
        vault_dependencies,
        &encoded_header_bytes,
        &vault_subkeys.vault_contents_key,
        &mut vault_contents,
    )?;

    Ok(UnlockedVault {
        vault_key,
        vault_header,
        encoded_header_bytes,
        vault_subkeys,
        vault_contents,
        last_activity_at: vault_dependencies.clock.now(),
    })
}
