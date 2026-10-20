use crate::application::vault_dependencies::VaultDependencies;
use crate::domain::encryption_key::EncryptionKey;
use crate::domain::key_derivation_parameters::KeyDerivationParameters;
use crate::domain::ports::authenticated_cipher::SealedBox;
use crate::domain::secret_text::SecretText;
use crate::domain::secure_random::random_bytes;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_file::vault_file_format::{MASTER_KEY_SALT_LENGTH_BYTES, VaultFileHeader};

/// Crea una cabecera nueva (salt nueva) con la clave de bóveda envuelta por la contraseña maestra.
pub fn build_header_wrapping_vault_key(
    vault_dependencies: &VaultDependencies,
    master_password: &SecretText,
    vault_key: &EncryptionKey,
    key_derivation_parameters: KeyDerivationParameters,
) -> VaultResult<VaultFileHeader> {
    let master_key_salt = random_bytes::<MASTER_KEY_SALT_LENGTH_BYTES>()?;
    let mut vault_header = VaultFileHeader {
        key_derivation_parameters,
        master_key_salt,
        wrapped_vault_key: SealedBox { nonce: Vec::new(), ciphertext_with_tag: Vec::new() },
    };

    let key_encryption_key = vault_dependencies.key_derivation_function.derive_key_encryption_key(
        &master_password.nfc_normalized(),
        &master_key_salt,
        &key_derivation_parameters,
    )?;
    vault_header.wrapped_vault_key = vault_dependencies.authenticated_cipher.seal(
        &key_encryption_key,
        vault_key.as_bytes(),
        &vault_header.key_wrapping_associated_data(),
    )?;
    Ok(vault_header)
}

/// Recupera la clave de bóveda. Cualquier fallo se reporta igual (no se distingue contraseña de manipulación).
///
/// Se prueba la contraseña en NFC y, si difiere, tal cual se escribió (bóvedas creadas antes de
/// normalizar): solo una contraseña con acentos descompuestos cuesta un segundo Argon2id.
pub fn unwrap_vault_key(
    vault_dependencies: &VaultDependencies,
    master_password: &SecretText,
    vault_header: &VaultFileHeader,
) -> VaultResult<EncryptionKey> {
    let mut unwrap_result = Err(VaultError::InvalidMasterPasswordOrCorruptedVault);
    for password_candidate in master_password.key_derivation_candidates() {
        unwrap_result = unwrap_vault_key_with_exact_password(vault_dependencies, &password_candidate, vault_header);
        if !matches!(unwrap_result, Err(VaultError::InvalidMasterPasswordOrCorruptedVault)) {
            break;
        }
    }
    unwrap_result
}

fn unwrap_vault_key_with_exact_password(
    vault_dependencies: &VaultDependencies,
    master_password: &SecretText,
    vault_header: &VaultFileHeader,
) -> VaultResult<EncryptionKey> {
    let key_encryption_key = vault_dependencies.key_derivation_function.derive_key_encryption_key(
        master_password,
        &vault_header.master_key_salt,
        &vault_header.key_derivation_parameters,
    )?;
    let vault_key_bytes = vault_dependencies
        .authenticated_cipher
        .open(&key_encryption_key, &vault_header.wrapped_vault_key, &vault_header.key_wrapping_associated_data())
        .map_err(|_| VaultError::InvalidMasterPasswordOrCorruptedVault)?;

    EncryptionKey::copy_from_slice(&vault_key_bytes).ok_or(VaultError::InvalidMasterPasswordOrCorruptedVault)
}
