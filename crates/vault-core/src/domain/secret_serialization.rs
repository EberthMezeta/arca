//! Serialización de estructuras con secretos sin dejar copias del texto plano en memoria.

use std::io::{self, Write};

use serde::Serialize;
use zeroize::Zeroizing;

use crate::domain::vault_error::{VaultError, VaultResult};

/// Cuenta bytes sin guardarlos: primera pasada para conocer el tamaño exacto.
#[derive(Default)]
struct SerializedByteCounter {
    counted_byte_count: usize,
}

impl Write for SerializedByteCounter {
    fn write(&mut self, written_bytes: &[u8]) -> io::Result<usize> {
        self.counted_byte_count += written_bytes.len();
        Ok(written_bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serializa a JSON en un buffer del tamaño exacto que se borra al soltarse.
///
/// `serde_json::to_vec` hace crecer el `Vec` duplicándolo: cada realocación libera el
/// bloque anterior con el texto plano dentro y sin borrar. Con dos pasadas (contar y
/// luego escribir) el buffer se reserva una sola vez.
pub fn serialize_secret_json<Value: Serialize>(serializable_value: &Value) -> VaultResult<Zeroizing<Vec<u8>>> {
    let mut serialized_byte_counter = SerializedByteCounter::default();
    serde_json::to_writer(&mut serialized_byte_counter, serializable_value).map_err(VaultError::storage)?;

    let mut serialized_bytes = Zeroizing::new(Vec::with_capacity(serialized_byte_counter.counted_byte_count));
    serde_json::to_writer(&mut *serialized_bytes, serializable_value).map_err(VaultError::storage)?;
    Ok(serialized_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_is_reserved_once_with_exact_size() {
        let serializable_value = vec!["contraseña-larga"; 500];

        let serialized_bytes = serialize_secret_json(&serializable_value).expect("serializar");

        assert_eq!(serialized_bytes.capacity(), serialized_bytes.len());
        assert_eq!(serialized_bytes.as_slice(), serde_json::to_vec(&serializable_value).expect("referencia").as_slice());
    }
}
