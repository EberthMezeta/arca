//! Lectura de archivos elegidos por el usuario sin cargar en memoria más de lo permitido
//! (un archivo de varios GB agotaría la RAM antes de poder rechazarlo).

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use zeroize::Zeroizing;

pub enum BoundedReadError {
    TooLarge,
    Unreadable(io::Error),
}

/// Lee como mucho `maximum_byte_count` bytes. El buffer se reserva con el tamaño del archivo
/// para no realocar (cada realocación dejaría una copia del contenido sin borrar).
pub fn read_file_with_size_limit(file_path: &Path, maximum_byte_count: u64) -> Result<Zeroizing<Vec<u8>>, BoundedReadError> {
    let opened_file = File::open(file_path).map_err(BoundedReadError::Unreadable)?;
    let declared_byte_count = opened_file.metadata().map_err(BoundedReadError::Unreadable)?.len();
    if declared_byte_count > maximum_byte_count {
        return Err(BoundedReadError::TooLarge);
    }

    let reserved_byte_count = usize::try_from(declared_byte_count).map_err(|_| BoundedReadError::TooLarge)?;
    let mut file_bytes = Zeroizing::new(Vec::with_capacity(reserved_byte_count));
    // `+ 1`: si el archivo creció tras consultar su tamaño, se detecta en vez de truncarlo.
    opened_file
        .take(maximum_byte_count + 1)
        .read_to_end(&mut file_bytes)
        .map_err(BoundedReadError::Unreadable)?;
    if file_bytes.len() as u64 > maximum_byte_count {
        return Err(BoundedReadError::TooLarge);
    }
    Ok(file_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_file_with(file_name: &str, file_bytes: &[u8]) -> std::path::PathBuf {
        let file_path = std::env::temp_dir().join(format!("arca-{}-{file_name}", std::process::id()));
        std::fs::write(&file_path, file_bytes).expect("escribir");
        file_path
    }

    #[test]
    fn small_file_is_read_completely() {
        let file_path = temporary_file_with("pequeno.csv", b"name,url\n");

        let read_result = read_file_with_size_limit(&file_path, 1024);

        assert!(matches!(read_result, Ok(file_bytes) if file_bytes.as_slice() == b"name,url\n"));
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn oversized_file_is_rejected_without_reading_it() {
        let file_path = temporary_file_with("grande.csv", &[b'x'; 2048]);

        let read_result = read_file_with_size_limit(&file_path, 1024);

        assert!(matches!(read_result, Err(BoundedReadError::TooLarge)));
        let _ = std::fs::remove_file(file_path);
    }
}
