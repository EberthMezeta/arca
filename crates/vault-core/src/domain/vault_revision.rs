//! Detección de rollback: cada guardado sube un contador que va cifrado (y autenticado) dentro
//! de la bóveda, y la app apunta aparte el último que ha visto. Si al abrir el archivo trae un
//! contador menor, alguien lo ha sustituido por una versión anterior (o se restauró una copia).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Qué bóveda es y cuántas veces se ha guardado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRevision {
    pub vault_id: Uuid,
    pub revision_number: u64,
}

/// La bóveda abierta es más antigua que la última que vio esta app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultRollback {
    pub opened_revision_number: u64,
    pub last_seen_revision_number: u64,
}

/// `opened` es `None` en bóvedas guardadas antes de existir el contador.
pub fn detect_rollback(last_seen: Option<VaultRevision>, opened: Option<VaultRevision>) -> Option<VaultRollback> {
    let last_seen = last_seen?;
    let opened_revision_number = match opened {
        // Lo apuntado ya tenía contador: una bóveda sin él es una copia anterior.
        None => 0,
        // Otra bóveda (p. ej. se creó una nueva): no es comparable.
        Some(opened) if opened.vault_id != last_seen.vault_id => return None,
        Some(opened) => opened.revision_number,
    };
    (opened_revision_number < last_seen.revision_number)
        .then_some(VaultRollback { opened_revision_number, last_seen_revision_number: last_seen.revision_number })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(vault_id: Uuid, revision_number: u64) -> VaultRevision {
        VaultRevision { vault_id, revision_number }
    }

    #[test]
    fn nothing_recorded_is_never_a_rollback() {
        assert_eq!(detect_rollback(None, Some(revision(Uuid::new_v4(), 0))), None);
        assert_eq!(detect_rollback(None, None), None);
    }

    #[test]
    fn same_or_newer_revision_is_accepted() {
        let vault_id = Uuid::new_v4();
        assert_eq!(detect_rollback(Some(revision(vault_id, 5)), Some(revision(vault_id, 5))), None);
        assert_eq!(detect_rollback(Some(revision(vault_id, 5)), Some(revision(vault_id, 9))), None);
    }

    #[test]
    fn older_revision_of_the_same_vault_is_a_rollback() {
        let vault_id = Uuid::new_v4();
        assert_eq!(
            detect_rollback(Some(revision(vault_id, 5)), Some(revision(vault_id, 3))),
            Some(VaultRollback { opened_revision_number: 3, last_seen_revision_number: 5 })
        );
    }

    #[test]
    fn copy_from_before_the_counter_existed_is_a_rollback() {
        assert_eq!(
            detect_rollback(Some(revision(Uuid::new_v4(), 2)), None),
            Some(VaultRollback { opened_revision_number: 0, last_seen_revision_number: 2 })
        );
    }

    #[test]
    fn a_different_vault_is_not_compared() {
        assert_eq!(detect_rollback(Some(revision(Uuid::new_v4(), 5)), Some(revision(Uuid::new_v4(), 1))), None);
    }
}
