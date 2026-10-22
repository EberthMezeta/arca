//! Puertos (inversión de dependencias): la aplicación depende de estos traits,
//! y la infraestructura los implementa.

pub mod attachment_storage;
pub mod authenticated_cipher;
pub mod clock;
pub mod key_derivation_function;
pub mod secret_clipboard;
pub mod unlock_throttle_store;
pub mod vault_exporter;
pub mod vault_importer;
pub mod vault_repository;
pub mod vault_revision_store;
