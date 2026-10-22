//! Endurecimiento del proceso al arrancar: la memoria contiene claves y secretos descifrados.
//!
//! - Sin volcados de memoria: un crash no debe dejar la bóveda descifrada en `/var/crash`
//!   (apport) ni en un `core` junto al ejecutable.
//! - Linux: `PR_SET_DUMPABLE = 0` impide además que otros procesos del mismo usuario lean
//!   la memoria con `ptrace` o `/proc/<pid>/mem` (como hacen `KeePassXC` o `ssh-agent`).
//!   La interfaz Slint se dibuja en este mismo proceso, así que también protege lo que se
//!   muestra al pulsar «Mostrar» (con Tauri, el proceso de `WebKit` quedaba fuera).

pub fn apply() {
    #[cfg(unix)]
    disable_core_dumps();
    #[cfg(target_os = "linux")]
    deny_memory_inspection();
}

/// Best-effort: si el sistema no lo permite, la app sigue funcionando.
#[cfg(unix)]
fn disable_core_dumps() {
    let no_core_dumps = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: `setrlimit` solo lee la estructura que se le pasa, que vive durante toda la llamada.
    unsafe {
        libc::setrlimit(libc::RLIMIT_CORE, &raw const no_core_dumps);
    }
}

#[cfg(target_os = "linux")]
fn deny_memory_inspection() {
    // SAFETY: `PR_SET_DUMPABLE` solo cambia un atributo del propio proceso; no toca memoria de Rust.
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
}
