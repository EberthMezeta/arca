//! Adaptadores de plataforma: todo lo que habla con el sistema operativo.

pub mod application_paths;
pub mod auto_lock_scheduler;
pub mod data_files;
pub mod file_dialogs;
pub mod legacy_data_migration;
pub mod local_user_files;
pub mod process_hardening;
pub mod single_instance_lock;
pub mod system_color_scheme;
pub mod system_lock_watcher;
pub mod system_secret_clipboard;
pub mod ui_preferences_store;
