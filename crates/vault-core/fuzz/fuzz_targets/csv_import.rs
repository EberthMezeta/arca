//! Importadores CSV (Bitwarden y navegadores): archivo elegido por el usuario, de origen desconocido.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_core::domain::ports::vault_importer::VaultImporter;
use vault_core::infrastructure::import_export::csv_transfer::{BitwardenCsvImporter, BrowserCsvImporter};

fuzz_target!(|file_bytes: &[u8]| {
    let _ = BitwardenCsvImporter.parse_file(file_bytes, None);
    let _ = BrowserCsvImporter.parse_file(file_bytes, None);
});
