use crate::application::manage_attachments::reencrypt_attachments_with_new_key;
use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_key_wrapping::{build_header_wrapping_vault_key, unwrap_vault_key};
use crate::application::vault_session::{UnlockedVault, seal_and_write_vault_file};
use crate::domain::key_hierarchy::derive_vault_subkeys;
use crate::domain::master_password_policy::MasterPasswordPolicy;
use crate::domain::secret_text::SecretText;
use crate::domain::secure_random::random_encryption_key;
use crate::domain::vault_error::{VaultError, VaultResult};

/// Cambia la contraseña maestra Y rota la clave de bóveda (salt nueva, parámetros KDF actuales).
///
/// Solo re-envolver la clave antigua no basta: quien tuviera una copia vieja del archivo
/// y la contraseña antigua (justo el motivo habitual para cambiarla) podría recuperar la
/// clave de bóveda y descifrar también las versiones futuras. Por eso se genera una clave
/// nueva, se vuelve a cifrar todo (adjuntos incluidos) y se escribe sin dejar copia `.bak`.
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

    let new_vault_key = random_encryption_key()?;
    let new_vault_subkeys = derive_vault_subkeys(&new_vault_key)?;
    let new_vault_header = build_header_wrapping_vault_key(
        vault_dependencies,
        new_master_password,
        &new_vault_key,
        vault_dependencies.key_derivation_policy.parameters_for_new_vaults,
    )?;
    let new_encoded_header_bytes = new_vault_header.encode()?;

    let mut rotated_contents = unlocked_vault.vault_contents.clone();
    let attachment_key_rotation = reencrypt_attachments_with_new_key(
        vault_dependencies,
        &unlocked_vault.vault_subkeys.attachments_root_key,
        &new_vault_subkeys.attachments_root_key,
        &mut rotated_contents,
    )?;
    let persist_result = seal_and_write_vault_file(
        vault_dependencies,
        &new_encoded_header_bytes,
        &new_vault_subkeys.vault_contents_key,
        &mut rotated_contents,
        |vault_file_bytes| vault_dependencies.vault_repository.write_vault_bytes_discarding_backups(vault_file_bytes),
    );
    if let Err(persist_error) = persist_result {
        attachment_key_rotation.discard_reencrypted_copies(vault_dependencies);
        return Err(persist_error);
    }

    // El cambio ya está guardado; borrar los adjuntos antiguos es best-effort
    // (un fallo aquí no debe hacer creer al usuario que la contraseña no cambió).
    attachment_key_rotation.discard_previous_copies(vault_dependencies);

    *unlocked_vault = UnlockedVault {
        vault_key: new_vault_key,
        vault_header: new_vault_header,
        encoded_header_bytes: new_encoded_header_bytes,
        vault_subkeys: new_vault_subkeys,
        vault_contents: rotated_contents,
        last_activity_at: unlocked_vault.last_activity_at,
    };
    Ok(())
}
