//! Núcleo del gestor de contraseñas: dominio, casos de uso e infraestructura.
//!
//! Capas (las dependencias solo apuntan hacia dentro):
//! - `domain`: entidades, valores y puertos (traits). Sin I/O.
//! - `application`: casos de uso orquestados por la fachada [`application::vault_service::VaultService`].
//! - `infrastructure`: adaptadores concretos (Argon2id, XChaCha20-Poly1305, ficheros, importadores...).

pub mod application;
pub mod domain;
pub mod infrastructure;
