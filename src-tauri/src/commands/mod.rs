mod attachment_commands;
mod clipboard_commands;
mod folder_commands;
mod generator_commands;
mod item_commands;
mod preference_commands;
mod session_commands;
mod transfer_commands;

use tauri::Wry;
use tauri::ipc::Invoke;

pub fn all_command_handlers() -> impl Fn(Invoke<Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        session_commands::get_vault_status,
        session_commands::create_vault,
        session_commands::unlock_vault,
        session_commands::lock_vault,
        session_commands::register_user_activity,
        session_commands::change_master_password,
        session_commands::get_vault_location,
        item_commands::search_vault_items,
        item_commands::get_vault_overview,
        item_commands::get_vault_item,
        item_commands::get_vault_item_details,
        item_commands::save_vault_item,
        item_commands::move_item_to_trash,
        item_commands::restore_item_from_trash,
        item_commands::delete_item_permanently,
        item_commands::empty_trash,
        item_commands::reveal_item_field,
        item_commands::get_totp_code,
        item_commands::get_password_health_report,
        folder_commands::list_folders,
        folder_commands::create_folder,
        folder_commands::rename_folder,
        folder_commands::delete_folder,
        clipboard_commands::copy_item_field,
        clipboard_commands::copy_totp_code,
        clipboard_commands::copy_text_secret,
        generator_commands::generate_password,
        generator_commands::estimate_password_strength,
        attachment_commands::add_attachment_from_file,
        attachment_commands::save_attachment_to_file,
        attachment_commands::delete_attachment,
        transfer_commands::import_vault_file,
        transfer_commands::export_vault_to_file,
        preference_commands::get_vault_preferences,
        preference_commands::update_vault_preferences,
    ]
}
