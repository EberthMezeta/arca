use chrono::{DateTime, Utc};

use crate::application::vault_dependencies::VaultDependencies;
use crate::domain::encryption_key::EncryptionKey;
use crate::domain::key_hierarchy::VaultSubkeys;
use crate::domain::ports::clock::Clock;
use crate::domain::secret_serialization::serialize_secret_json;
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

/// Última actividad medida con los dos relojes: el de pared cuenta el tiempo de suspensión y
/// el monótono no se deja engañar si alguien (o NTP) atrasa la hora del sistema.
#[derive(Debug, Clone, Copy)]
pub struct ActivityTimestamp {
    wall_clock: DateTime<Utc>,
    monotonic_clock: std::time::Duration,
}

impl ActivityTimestamp {
    pub fn now(clock: &dyn Clock) -> Self {
        Self { wall_clock: clock.now(), monotonic_clock: clock.monotonic_now() }
    }

    /// El mayor de los dos: basta con que uno diga que pasó el tiempo para bloquear.
    pub fn idle_seconds(&self, clock: &dyn Clock) -> u64 {
        let wall_clock_idle_seconds = u64::try_from((clock.now() - self.wall_clock).num_seconds()).unwrap_or(0);
        let monotonic_idle_seconds = clock.monotonic_now().saturating_sub(self.monotonic_clock).as_secs();
        wall_clock_idle_seconds.max(monotonic_idle_seconds)
    }
}

pub struct UnlockedVault {
    pub vault_key: EncryptionKey,
    pub vault_header: VaultFileHeader,
    pub encoded_header_bytes: Vec<u8>,
    pub vault_subkeys: VaultSubkeys,
    pub vault_contents: VaultContents,
    pub last_activity: ActivityTimestamp,
}

impl UnlockedVault {
    /// Aplica el cambio sobre una copia, la guarda cifrada y solo entonces la adopta.
    /// Si el guardado falla, el estado en memoria sigue igual que el disco.
    pub fn modify_and_persist<MutationOutput>(
        &mut self,
        vault_dependencies: &VaultDependencies,
        apply_mutation: impl FnOnce(&mut VaultContents) -> VaultResult<MutationOutput>,
    ) -> VaultResult<MutationOutput> {
        self.modify_and_write(vault_dependencies, apply_mutation, |vault_file_bytes| {
            vault_dependencies.vault_repository.write_vault_bytes(vault_file_bytes)
        })
    }

    /// Igual que `modify_and_persist`, pero sin dejar la versión anterior como `.bak`: para
    /// borrados definitivos, que si no seguirían recuperables desde la copia con la misma contraseña.
    pub fn modify_and_persist_discarding_backups<MutationOutput>(
        &mut self,
        vault_dependencies: &VaultDependencies,
        apply_mutation: impl FnOnce(&mut VaultContents) -> VaultResult<MutationOutput>,
    ) -> VaultResult<MutationOutput> {
        self.modify_and_write(vault_dependencies, apply_mutation, |vault_file_bytes| {
            vault_dependencies.vault_repository.write_vault_bytes_discarding_backups(vault_file_bytes)
        })
    }

    fn modify_and_write<MutationOutput>(
        &mut self,
        vault_dependencies: &VaultDependencies,
        apply_mutation: impl FnOnce(&mut VaultContents) -> VaultResult<MutationOutput>,
        write_vault_file: impl FnOnce(&[u8]) -> VaultResult<()>,
    ) -> VaultResult<MutationOutput> {
        let mut updated_contents = self.vault_contents.clone();
        let mutation_output = apply_mutation(&mut updated_contents)?;
        seal_and_write_vault_file(
            vault_dependencies,
            &self.encoded_header_bytes,
            &self.vault_subkeys.vault_contents_key,
            &mut updated_contents,
            write_vault_file,
        )?;
        self.vault_contents = updated_contents;
        Ok(mutation_output)
    }
}

/// Serializa, cifra (AAD = cabecera) y escribe la bóveda completa, conservando la versión anterior como `.bak`.
pub fn persist_vault_contents(
    vault_dependencies: &VaultDependencies,
    encoded_header_bytes: &[u8],
    vault_contents_key: &EncryptionKey,
    vault_contents: &mut VaultContents,
) -> VaultResult<()> {
    seal_and_write_vault_file(
        vault_dependencies,
        encoded_header_bytes,
        vault_contents_key,
        vault_contents,
        |vault_file_bytes| vault_dependencies.vault_repository.write_vault_bytes(vault_file_bytes),
    )
}

/// Sube la revisión, cifra, escribe con `write_vault_file` y apunta la revisión como la última vista.
/// Si apuntarla falla no pasa nada grave: el siguiente desbloqueo ve una revisión mayor, no menor.
pub fn seal_and_write_vault_file(
    vault_dependencies: &VaultDependencies,
    encoded_header_bytes: &[u8],
    vault_contents_key: &EncryptionKey,
    vault_contents: &mut VaultContents,
    write_vault_file: impl FnOnce(&[u8]) -> VaultResult<()>,
) -> VaultResult<()> {
    let vault_revision = vault_contents.advance_revision();
    let vault_file_bytes =
        seal_vault_file(vault_dependencies, encoded_header_bytes, vault_contents_key, vault_contents)?;
    write_vault_file(&vault_file_bytes)?;
    let _ = vault_dependencies.vault_revision_store.record_revision(&vault_revision);
    Ok(())
}

/// Serializa y cifra la bóveda completa (AAD = cabecera) sin escribirla.
fn seal_vault_file(
    vault_dependencies: &VaultDependencies,
    encoded_header_bytes: &[u8],
    vault_contents_key: &EncryptionKey,
    vault_contents: &VaultContents,
) -> VaultResult<Vec<u8>> {
    let serialized_contents = serialize_secret_json(vault_contents)?;
    let encrypted_contents =
        vault_dependencies.authenticated_cipher.seal(vault_contents_key, &serialized_contents, encoded_header_bytes)?;
    encode_vault_file(encoded_header_bytes, &encrypted_contents)
}
