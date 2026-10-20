use tauri::State;
use vault_core::domain::password_generation::password_generation_options::PasswordGenerationOptions;
use vault_core::domain::password_strength::PasswordStrength;
use vault_core::domain::secret_text::SecretText;

use crate::app_state::AppState;
use crate::command_error::CommandResult;

#[tauri::command]
pub fn generate_password(
    app_state: State<'_, AppState>,
    generation_options: PasswordGenerationOptions,
) -> CommandResult<SecretText> {
    Ok(app_state.lock_vault_service()?.generate_password(generation_options)?)
}

#[tauri::command]
pub fn estimate_password_strength(
    app_state: State<'_, AppState>,
    candidate_password: SecretText,
) -> CommandResult<PasswordStrength> {
    Ok(app_state.lock_vault_service()?.estimate_password_strength(&candidate_password))
}
