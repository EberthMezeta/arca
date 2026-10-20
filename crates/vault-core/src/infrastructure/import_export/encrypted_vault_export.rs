//! Copia de seguridad portátil cifrada con una contraseña propia del archivo
//! (Argon2id + XChaCha20-Poly1305). Formato JSON legible para facilitar futuras migraciones.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde::{Deserialize, Serialize};

use crate::domain::key_derivation_parameters::KeyDerivationParameters;
use crate::domain::master_password_policy::MasterPasswordPolicy;
use crate::domain::ports::authenticated_cipher::{AuthenticatedCipher, SealedBox};
use crate::domain::ports::key_derivation_function::KeyDerivationFunction;
use crate::domain::ports::vault_exporter::{ExportFormat, VaultExporter};
use crate::domain::ports::vault_importer::{ImportFormat, ImportedVaultData, VaultImporter};
use crate::domain::secret_serialization::serialize_secret_json;
use crate::domain::secret_text::SecretText;
use crate::domain::secure_random::random_bytes;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::infrastructure::crypto::argon2id_key_derivation::Argon2idKeyDerivation;
use crate::infrastructure::crypto::xchacha20_poly1305_cipher::XChaCha20Poly1305Cipher;

const EXPORT_FORMAT_IDENTIFIER: &str = "rustvault-encrypted-export";
const EXPORT_FORMAT_VERSION: u16 = 1;
const EXPORT_SALT_LENGTH_BYTES: usize = 32;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedExportEnvelope {
    format_identifier: String,
    format_version: u16,
    key_derivation_parameters: KeyDerivationParameters,
    salt_base64: String,
    nonce_base64: String,
    ciphertext_base64: String,
}

impl EncryptedExportEnvelope {
    /// Todo lo que va en claro se autentica: cambiar parámetros o salt invalida el archivo.
    fn associated_data(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.format_identifier,
            self.format_version,
            self.key_derivation_parameters.memory_cost_kibibytes,
            self.key_derivation_parameters.iteration_count,
            self.key_derivation_parameters.parallelism_degree,
            self.salt_base64
        )
        .into_bytes()
    }
}

fn decode_base64(encoded_text: &str) -> VaultResult<Vec<u8>> {
    BASE64_STANDARD.decode(encoded_text).map_err(VaultError::import_failed)
}

pub struct EncryptedVaultExportExporter;

impl VaultExporter for EncryptedVaultExportExporter {
    fn export_format(&self) -> ExportFormat {
        ExportFormat::EncryptedVaultExport
    }

    fn requires_file_password(&self) -> bool {
        true
    }

    fn suggested_file_extension(&self) -> &'static str {
        "rvexport"
    }

    fn serialize_vault(&self, vault_contents: &VaultContents, file_password: Option<&SecretText>) -> VaultResult<Vec<u8>> {
        let file_password = file_password.ok_or(VaultError::TransferPasswordRequired)?;
        let violations = MasterPasswordPolicy::default().find_violations(file_password);
        if !violations.is_empty() {
            return Err(VaultError::WeakMasterPassword { violations });
        }

        let export_salt = random_bytes::<EXPORT_SALT_LENGTH_BYTES>()?;
        let mut export_envelope = EncryptedExportEnvelope {
            format_identifier: EXPORT_FORMAT_IDENTIFIER.to_string(),
            format_version: EXPORT_FORMAT_VERSION,
            key_derivation_parameters: KeyDerivationParameters::RECOMMENDED,
            salt_base64: BASE64_STANDARD.encode(export_salt),
            nonce_base64: String::new(),
            ciphertext_base64: String::new(),
        };

        let export_key = Argon2idKeyDerivation.derive_key_encryption_key(
            file_password,
            &export_salt,
            &export_envelope.key_derivation_parameters,
        )?;
        let serialized_contents = serialize_secret_json(vault_contents)?;
        let sealed_contents =
            XChaCha20Poly1305Cipher.seal(&export_key, &serialized_contents, &export_envelope.associated_data())?;

        export_envelope.nonce_base64 = BASE64_STANDARD.encode(&sealed_contents.nonce);
        export_envelope.ciphertext_base64 = BASE64_STANDARD.encode(&sealed_contents.ciphertext_with_tag);
        serde_json::to_vec_pretty(&export_envelope).map_err(VaultError::storage)
    }
}

pub struct EncryptedVaultExportImporter;

impl VaultImporter for EncryptedVaultExportImporter {
    fn import_format(&self) -> ImportFormat {
        ImportFormat::EncryptedVaultExport
    }

    fn parse_file(&self, file_bytes: &[u8], file_password: Option<&SecretText>) -> VaultResult<ImportedVaultData> {
        let file_password = file_password.ok_or(VaultError::TransferPasswordRequired)?;
        let export_envelope: EncryptedExportEnvelope =
            serde_json::from_slice(file_bytes).map_err(VaultError::import_failed)?;
        if export_envelope.format_identifier != EXPORT_FORMAT_IDENTIFIER {
            return Err(VaultError::import_failed("no es una copia cifrada de esta aplicación"));
        }
        if export_envelope.format_version != EXPORT_FORMAT_VERSION {
            return Err(VaultError::UnsupportedVaultVersion { found_version: export_envelope.format_version });
        }

        let has_safe_parameters = export_envelope.key_derivation_parameters.is_within(
            &KeyDerivationParameters::MINIMUM_ACCEPTED,
            &KeyDerivationParameters::MAXIMUM_ACCEPTED,
        );
        if !has_safe_parameters {
            return Err(VaultError::InsecureKeyDerivationParameters);
        }

        let export_salt = decode_base64(&export_envelope.salt_base64)?;
        let sealed_contents = SealedBox {
            nonce: decode_base64(&export_envelope.nonce_base64)?,
            ciphertext_with_tag: decode_base64(&export_envelope.ciphertext_base64)?,
        };
        let export_key = Argon2idKeyDerivation.derive_key_encryption_key(
            file_password,
            &export_salt,
            &export_envelope.key_derivation_parameters,
        )?;
        let serialized_contents = XChaCha20Poly1305Cipher
            .open(&export_key, &sealed_contents, &export_envelope.associated_data())
            .map_err(|_| VaultError::InvalidMasterPasswordOrCorruptedVault)?;

        let exported_contents: VaultContents =
            serde_json::from_slice(&serialized_contents).map_err(VaultError::import_failed)?;
        Ok(ImportedVaultData { folders: exported_contents.folders, vault_items: exported_contents.vault_items })
    }
}
