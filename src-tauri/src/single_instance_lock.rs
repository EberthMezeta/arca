//! Solo puede haber una instancia abierta: cada una guarda su copia completa de la bóveda,
//! así que con dos a la vez la última en guardar pisaría en silencio los cambios de la otra.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

const SINGLE_INSTANCE_LOCK_FILE_NAME: &str = "rustvault.lock";

pub enum InstanceLockOutcome {
    /// El bloqueo dura lo que viva este `File`: el sistema lo libera al cerrarse la app (aunque se cuelgue).
    Acquired(File),
    HeldByAnotherInstance,
}

pub fn acquire(application_data_directory: &Path) -> io::Result<InstanceLockOutcome> {
    let lock_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(application_data_directory.join(SINGLE_INSTANCE_LOCK_FILE_NAME))?;
    match lock_file.try_lock() {
        Ok(()) => Ok(InstanceLockOutcome::Acquired(lock_file)),
        Err(TryLockError::WouldBlock) => Ok(InstanceLockOutcome::HeldByAnotherInstance),
        Err(TryLockError::Error(lock_error)) => Err(lock_error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_cannot_take_the_lock_until_the_first_releases_it() {
        let temporary_directory = std::env::temp_dir().join(format!("rustvault-lock-test-{}", std::process::id()));
        std::fs::create_dir_all(&temporary_directory).expect("crear directorio");

        let first_instance = acquire(&temporary_directory).expect("primer bloqueo");
        assert!(matches!(first_instance, InstanceLockOutcome::Acquired(_)));
        assert!(matches!(acquire(&temporary_directory).expect("segundo intento"), InstanceLockOutcome::HeldByAnotherInstance));

        drop(first_instance);
        assert!(matches!(acquire(&temporary_directory).expect("tras liberar"), InstanceLockOutcome::Acquired(_)));
        let _ = std::fs::remove_dir_all(&temporary_directory);
    }
}
