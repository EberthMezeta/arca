//! Papelera: mover, restaurar (deshacer) y borrar definitivamente.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::VaultResult;
use crate::domain::vault_item::VaultItem;

pub fn move_to_trash(vault_contents: &mut VaultContents, item_id: Uuid, now: DateTime<Utc>) -> VaultResult<()> {
    let vault_item = vault_contents.find_item_mut(item_id)?;
    vault_item.moved_to_trash_at = Some(now);
    vault_item.updated_at = now;
    Ok(())
}

pub fn restore_from_trash(vault_contents: &mut VaultContents, item_id: Uuid, now: DateTime<Utc>) -> VaultResult<()> {
    let vault_item = vault_contents.find_item_mut(item_id)?;
    vault_item.moved_to_trash_at = None;
    vault_item.updated_at = now;
    Ok(())
}

/// Devuelve los elementos eliminados para que el llamador borre sus adjuntos cifrados.
pub fn delete_permanently(vault_contents: &mut VaultContents, item_id: Uuid) -> VaultResult<Vec<VaultItem>> {
    vault_contents.remove_item(item_id).map(|removed_item| vec![removed_item])
}

pub fn empty_trash(vault_contents: &mut VaultContents) -> Vec<VaultItem> {
    let (trashed_items, active_items): (Vec<VaultItem>, Vec<VaultItem>) =
        vault_contents.vault_items.drain(..).partition(VaultItem::is_in_trash);
    vault_contents.vault_items = active_items;
    trashed_items
}
