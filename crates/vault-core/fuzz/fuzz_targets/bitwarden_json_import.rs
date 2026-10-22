//! Importador de exportaciones JSON de Bitwarden: archivo elegido por el usuario, de origen desconocido.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_core::domain::ports::vault_importer::VaultImporter;
use vault_core::infrastructure::import_export::bitwarden_json_transfer::BitwardenJsonImporter;

fuzz_target!(|file_bytes: &[u8]| {
    let _ = BitwardenJsonImporter.parse_file(file_bytes, None);
});
