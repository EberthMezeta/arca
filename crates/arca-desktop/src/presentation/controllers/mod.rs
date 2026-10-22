//! Un controlador por zona de la interfaz. Se comunican solo a través de `AppContext`
//! (navegador del espacio de trabajo y bus de eventos), nunca directamente entre sí.

pub mod clipboard_feedback;
pub mod command_palette_controller;
#[cfg(test)]
mod controller_tests;
pub mod dialog_controller;
pub mod folder_dialogs;
pub mod generator_controller;
pub mod health_controller;
pub mod i18n_controller;
pub mod item_details_controller;
pub mod item_editor_controller;
pub mod session_controller;
pub mod settings_controller;
pub mod toast_controller;
pub mod workspace_controller;

use std::rc::Rc;

use crate::AppWindow;
use crate::presentation::app_context::AppContext;

/// Conecta todos los controladores con la ventana. Los controladores viven mientras la ventana
/// (cada callback de Slint guarda un `Rc` al suyo).
pub fn install_all(context: &Rc<AppContext>, app_window: &AppWindow) {
    context.toasts.install(app_window);
    context.dialogs.install(app_window);
    i18n_controller::install(context, app_window);
    settings_controller::SettingsController::install(context, app_window);
    generator_controller::GeneratorController::install(context, app_window);
    workspace_controller::WorkspaceController::install(context, app_window);
    item_details_controller::ItemDetailsController::install(context, app_window);
    item_editor_controller::ItemEditorController::install(context, app_window);
    health_controller::HealthController::install(context, app_window);
    command_palette_controller::CommandPaletteController::install(context, app_window);
    session_controller::SessionController::install(context, app_window);
}
