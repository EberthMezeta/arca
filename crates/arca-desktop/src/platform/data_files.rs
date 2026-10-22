//! Nombres de lo que la app guarda en su carpeta de datos (una sola lista para la raíz de
//! composición y para la migración desde `RustVault`).

pub const VAULT_FILE_NAME: &str = "personal.vault";
/// La copia que deja `AtomicFileVaultRepository` al sobrescribir la bóveda.
pub const VAULT_BACKUP_FILE_NAME: &str = "personal.vault.bak";
pub const ATTACHMENTS_DIRECTORY_NAME: &str = "personal.vault.d";
pub const UNLOCK_THROTTLE_FILE_NAME: &str = "unlock-throttle.json";
pub const VAULT_REVISION_FILE_NAME: &str = "vault-revision.json";
pub const UI_PREFERENCES_FILE_NAME: &str = "ui-preferences.json";
pub const SINGLE_INSTANCE_LOCK_FILE_NAME: &str = "arca.lock";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_name_matches_what_the_repository_writes() {
        assert_eq!(VAULT_BACKUP_FILE_NAME, format!("{VAULT_FILE_NAME}.bak"));
    }
}
