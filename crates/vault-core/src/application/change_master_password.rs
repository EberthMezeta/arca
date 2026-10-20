use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_key_wrapping::{build_header_wrapping_vault_key, unwrap_vault_key};
use crate::application::vault_session::{UnlockedVault, persist_vault_contents};
use crate::domain::master_password_policy::MasterPasswordPolicy;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::{VaultError, VaultResult};

/// Re-envuelve la misma clave de bóveda con la nueva contraseña (salt nueva, parámetros KDF actuales).
/// No hace falta re-cifrar los adjuntos.
pub fn execute(
    vault_dependencies: &VaultDependencies,
    unlocked_vault: &mut UnlockedVault,
    current_master_password: &SecretText,
    new_master_password: &SecretText,
) -> VaultResult<()> {
    unwrap_vault_key(vault_dependencies, current_master_password, &unlocked_vault.vault_header)?;
    let violations = MasterPasswordPolicy::default().find_violations(new_master_password);
    if !violations.is_empty() {
        return Err(VaultError::WeakMasterPassword { violations });
    }

    let new_vault_header = build_header_wrapping_vault_key(
        vault_dependencies,
        new_master_password,
        &unlocked_vault.vault_key,
        vault_dependencies.key_derivation_policy.parameters_for_new_vaults,
    )?;
    let new_encoded_header_bytes = new_vault_header.encode()?;
    persist_vault_contents(
        vault_dependencies,
        &new_encoded_header_bytes,
        &unlocked_vault.vault_subkeys.vault_contents_key,
        &unlocked_vault.vault_contents,
    )?;

    unlocked_vault.vault_header = new_vault_header;
    unlocked_vault.encoded_header_bytes = new_encoded_header_bytes;
    Ok(())
}
