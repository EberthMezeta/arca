//! Lector del archivo `.vault`: lo primero que se procesa de un archivo que puede haber cambiado
//! cualquiera con acceso al disco, antes de cualquier comprobación criptográfica.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_core::domain::vault_file::vault_file_format::{decode_standalone_sealed_box, decode_vault_file};

fuzz_target!(|file_bytes: &[u8]| {
    if let Ok(decoded_vault_file) = decode_vault_file(file_bytes) {
        // Lo decodificado tiene que volver a codificarse igual (salvo el contenido, que va tal cual).
        let encoded_header_bytes =
            decoded_vault_file.vault_header.encode().expect("una cabecera leída se puede escribir");
        assert_eq!(encoded_header_bytes, decoded_vault_file.encoded_header_bytes);
    }
    let _ = decode_standalone_sealed_box(file_bytes);
});
