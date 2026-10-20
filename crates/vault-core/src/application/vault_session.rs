use chrono::{DateTime, Utc};
use zeroize::Zeroizing;

use crate::application::vault_dependencies::VaultDependencies;
use crate::domain::encryption_key::EncryptionKey;
use crate::domain::key_hierarchy::VaultSubkeys;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_file::vault_file_format::{VaultFileHeader, encode_vault_file};

/// Patrón State: o la bóveda está bloqueada (sin ninguna clave en memoria) o desbloqueada.
#[derive(Default)]
pub enum VaultSessionState {
    #[default]
    Locked,
    Unlocked(Box<UnlockedVault>),
}

impl VaultSessionState {
    pub fn is_unlocked(&self) -> bool {
        matches!(self, Self::Unlocked(_))
    }

    pub fn require_unlocked(&self) -> VaultResult<&UnlockedVault> {
        match self {
            Self::Unlocked(unlocked_vault) => Ok(unlocked_vault),
            Self::Locked => Err(VaultError::VaultLocked),
        }
    }

    pub fn require_unlocked_mut(&mut self) -> VaultResult<&mut UnlockedVault> {
        match self {
            Self::Unlocked(unlocked_vault) => Ok(unlocked_vault),
            Self::Locked => Err(VaultError::VaultLocked),
        }
    }

    /// Al bloquear se descarta el estado desbloqueado: `Zeroize` borra claves y secretos.
    pub fn lock(&mut self) {
        *self = Self::Locked;
    }
}

pub struct UnlockedVault {
    pub vault_key: EncryptionKey,
    pub vault_header: VaultFileHeader,
    pub encoded_header_bytes: Vec<u8>,
    pub vault_subkeys: VaultSubkeys,
    pub vault_contents: VaultContents,
    pub last_activity_at: DateTime<Utc>,
}

impl UnlockedVault {
    /// Aplica el cambio sobre una copia, la guarda cifrada y solo entonces la adopta.
    /// Si el guardado falla, el estado en memoria sigue igual que el disco.
    pub fn modify_and_persist<MutationOutput>(
        &mut self,
        vault_dependencies: &VaultDependencies,
        apply_mutation: impl FnOnce(&mut VaultContents) -> VaultResult<MutationOutput>,
    ) -> VaultResult<MutationOutput> {
        let mut updated_contents = self.vault_contents.clone();
        let mutation_output = apply_mutation(&mut updated_contents)?;
        persist_vault_contents(
            vault_dependencies,
            &self.encoded_header_bytes,
            &self.vault_subkeys.vault_contents_key,
            &updated_contents,
        )?;
        self.vault_contents = updated_contents;
        Ok(mutation_output)
    }
}

/// Serializa, cifra (AAD = cabecera) y escribe la bóveda completa.
pub fn persist_vault_contents(
    vault_dependencies: &VaultDependencies,
    encoded_header_bytes: &[u8],
    vault_contents_key: &EncryptionKey,
    vault_contents: &VaultContents,
) -> VaultResult<()> {
    let serialized_contents = Zeroizing::new(serde_json::to_vec(vault_contents).map_err(|_| VaultError::Cryptography)?);
    let encrypted_contents =
        vault_dependencies.authenticated_cipher.seal(vault_contents_key, &serialized_contents, encoded_header_bytes)?;
    let vault_file_bytes = encode_vault_file(encoded_header_bytes, &encrypted_contents)?;
    vault_dependencies.vault_repository.write_vault_bytes(&vault_file_bytes)
}
