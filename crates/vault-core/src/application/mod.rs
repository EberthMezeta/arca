pub mod analyze_password_health;
pub mod change_master_password;
pub mod create_vault;
pub mod import_export_vault;
pub mod manage_attachments;
pub mod manage_folders;
pub mod manage_item_lifecycle;
pub mod save_vault_item;
pub mod search_vault_items;
pub mod throttled_master_password_attempt;
pub mod transfer_format_registry;
pub mod unlock_vault;
pub mod vault_dependencies;
pub mod vault_key_wrapping;
pub mod vault_service;
pub mod vault_session;
/// Bóveda en memoria con Argon2id barato, para los tests del núcleo y de la app (`test-support`).
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
#[cfg(test)]
mod vault_service_tests;
