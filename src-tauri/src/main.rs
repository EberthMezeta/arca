// Oculta la consola en Windows en builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    arca_lib::run();
}
