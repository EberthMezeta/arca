fn main() {
    // Fluent en todas las plataformas: la UI se ve igual en Linux, Windows y macOS.
    let compiler_configuration = slint_build::CompilerConfiguration::new().with_style("fluent".into());
    slint_build::compile_with_config("ui/app_window.slint", compiler_configuration)
        .expect("compilar la interfaz Slint");
}
