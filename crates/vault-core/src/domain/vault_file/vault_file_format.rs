//! Formato binario del archivo `.vault` (little-endian):
//!
//! ```text
//! ┌──────────────── cabecera (AAD del contenido) ──────────────────────────────┐
//! │ "RVLT" │ versión u16 │ algoritmo KDF u8 │ memoria u32 │ iteraciones u32 │    │
//! │ paralelismo u32 │ salt [32] │ len u8 + nonce │ len u16 + clave envuelta      │
//! └────────────────────────────────────────────────────────────────────────────┘
//! │ len u8 + nonce del contenido │ contenido cifrado (resto del archivo)       │
//! ```
//! La clave de bóveda envuelta usa como AAD la cabecera hasta la salt, así que
//! cambiar los parámetros KDF o la salt invalida el archivo.

use crate::domain::key_derivation_parameters::KeyDerivationParameters;
use crate::domain::ports::authenticated_cipher::SealedBox;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_file::byte_reader::ByteReader;

pub const VAULT_FILE_MAGIC: &[u8; 4] = b"RVLT";
pub const CURRENT_VAULT_FORMAT_VERSION: u16 = 1;
pub const MASTER_KEY_SALT_LENGTH_BYTES: usize = 32;
const ARGON2ID_ALGORITHM_IDENTIFIER: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultFileHeader {
    pub key_derivation_parameters: KeyDerivationParameters,
    pub master_key_salt: [u8; MASTER_KEY_SALT_LENGTH_BYTES],
    pub wrapped_vault_key: SealedBox,
}

#[derive(Debug)]
pub struct DecodedVaultFile {
    pub vault_header: VaultFileHeader,
    pub encoded_header_bytes: Vec<u8>,
    pub encrypted_contents: SealedBox,
}

impl VaultFileHeader {
    /// Bytes autenticados al envolver la clave de bóveda (todo lo anterior a la clave envuelta).
    pub fn key_wrapping_associated_data(&self) -> Vec<u8> {
        let mut associated_data = Vec::with_capacity(64);
        associated_data.extend_from_slice(VAULT_FILE_MAGIC);
        associated_data.extend_from_slice(&CURRENT_VAULT_FORMAT_VERSION.to_le_bytes());
        associated_data.push(ARGON2ID_ALGORITHM_IDENTIFIER);
        associated_data.extend_from_slice(&self.key_derivation_parameters.memory_cost_kibibytes.to_le_bytes());
        associated_data.extend_from_slice(&self.key_derivation_parameters.iteration_count.to_le_bytes());
        associated_data.extend_from_slice(&self.key_derivation_parameters.parallelism_degree.to_le_bytes());
        associated_data.extend_from_slice(&self.master_key_salt);
        associated_data
    }

    pub fn encode(&self) -> VaultResult<Vec<u8>> {
        let mut encoded_header = self.key_wrapping_associated_data();
        append_length_prefixed_u8(&mut encoded_header, &self.wrapped_vault_key.nonce)?;
        append_length_prefixed_u16(&mut encoded_header, &self.wrapped_vault_key.ciphertext_with_tag)?;
        Ok(encoded_header)
    }
}

pub fn encode_vault_file(encoded_header_bytes: &[u8], encrypted_contents: &SealedBox) -> VaultResult<Vec<u8>> {
    let mut vault_file_bytes = encoded_header_bytes.to_vec();
    append_length_prefixed_u8(&mut vault_file_bytes, &encrypted_contents.nonce)?;
    vault_file_bytes.extend_from_slice(&encrypted_contents.ciphertext_with_tag);
    Ok(vault_file_bytes)
}

pub fn decode_vault_file(vault_file_bytes: &[u8]) -> VaultResult<DecodedVaultFile> {
    let mut byte_reader = ByteReader::new(vault_file_bytes);

    let file_magic = byte_reader.read_array::<4>()?;
    if &file_magic != VAULT_FILE_MAGIC {
        return Err(VaultError::corrupted_file("no es un archivo de bóveda"));
    }
    let format_version = byte_reader.read_u16_le()?;
    if format_version != CURRENT_VAULT_FORMAT_VERSION {
        return Err(VaultError::UnsupportedVaultVersion { found_version: format_version });
    }
    if byte_reader.read_u8()? != ARGON2ID_ALGORITHM_IDENTIFIER {
        return Err(VaultError::corrupted_file("algoritmo KDF desconocido"));
    }

    let key_derivation_parameters = KeyDerivationParameters {
        memory_cost_kibibytes: byte_reader.read_u32_le()?,
        iteration_count: byte_reader.read_u32_le()?,
        parallelism_degree: byte_reader.read_u32_le()?,
    };
    let master_key_salt = byte_reader.read_array::<MASTER_KEY_SALT_LENGTH_BYTES>()?;
    let wrapped_vault_key = read_sealed_box_u8_u16(&mut byte_reader)?;
    let encoded_header_bytes = vault_file_bytes[..byte_reader.read_position()].to_vec();

    let contents_nonce_length = usize::from(byte_reader.read_u8()?);
    let contents_nonce = byte_reader.read_slice(contents_nonce_length)?.to_vec();
    let contents_ciphertext = byte_reader.read_remaining().to_vec();

    Ok(DecodedVaultFile {
        vault_header: VaultFileHeader { key_derivation_parameters, master_key_salt, wrapped_vault_key },
        encoded_header_bytes,
        encrypted_contents: SealedBox { nonce: contents_nonce, ciphertext_with_tag: contents_ciphertext },
    })
}

fn read_sealed_box_u8_u16(byte_reader: &mut ByteReader<'_>) -> VaultResult<SealedBox> {
    let nonce_length = usize::from(byte_reader.read_u8()?);
    let nonce = byte_reader.read_slice(nonce_length)?.to_vec();
    let ciphertext_length = usize::from(byte_reader.read_u16_le()?);
    let ciphertext_with_tag = byte_reader.read_slice(ciphertext_length)?.to_vec();
    Ok(SealedBox { nonce, ciphertext_with_tag })
}

fn append_length_prefixed_u8(target_buffer: &mut Vec<u8>, field_bytes: &[u8]) -> VaultResult<()> {
    let field_length = u8::try_from(field_bytes.len()).map_err(|_| VaultError::Cryptography)?;
    target_buffer.push(field_length);
    target_buffer.extend_from_slice(field_bytes);
    Ok(())
}

fn append_length_prefixed_u16(target_buffer: &mut Vec<u8>, field_bytes: &[u8]) -> VaultResult<()> {
    let field_length = u16::try_from(field_bytes.len()).map_err(|_| VaultError::Cryptography)?;
    target_buffer.extend_from_slice(&field_length.to_le_bytes());
    target_buffer.extend_from_slice(field_bytes);
    Ok(())
}

/// Codifica un `SealedBox` independiente (p. ej. un adjunto): `len u8 + nonce | texto cifrado`.
pub fn encode_standalone_sealed_box(sealed_box: &SealedBox) -> VaultResult<Vec<u8>> {
    let mut encoded_bytes = Vec::with_capacity(1 + sealed_box.nonce.len() + sealed_box.ciphertext_with_tag.len());
    append_length_prefixed_u8(&mut encoded_bytes, &sealed_box.nonce)?;
    encoded_bytes.extend_from_slice(&sealed_box.ciphertext_with_tag);
    Ok(encoded_bytes)
}

pub fn decode_standalone_sealed_box(encoded_bytes: &[u8]) -> VaultResult<SealedBox> {
    let mut byte_reader = ByteReader::new(encoded_bytes);
    let nonce_length = usize::from(byte_reader.read_u8()?);
    let nonce = byte_reader.read_slice(nonce_length)?.to_vec();
    Ok(SealedBox { nonce, ciphertext_with_tag: byte_reader.read_remaining().to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_header() -> VaultFileHeader {
        VaultFileHeader {
            key_derivation_parameters: KeyDerivationParameters::RECOMMENDED,
            master_key_salt: [7u8; MASTER_KEY_SALT_LENGTH_BYTES],
            wrapped_vault_key: SealedBox { nonce: vec![1; 24], ciphertext_with_tag: vec![2; 48] },
        }
    }

    #[test]
    fn encoded_file_decodes_to_same_parts() {
        let vault_header = sample_header();
        let encoded_header_bytes = vault_header.encode().expect("cabecera");
        let encrypted_contents = SealedBox { nonce: vec![3; 24], ciphertext_with_tag: vec![4; 100] };

        let vault_file_bytes = encode_vault_file(&encoded_header_bytes, &encrypted_contents).expect("archivo");
        let decoded_vault_file = decode_vault_file(&vault_file_bytes).expect("decodificar");

        assert_eq!(decoded_vault_file.vault_header, vault_header);
        assert_eq!(decoded_vault_file.encoded_header_bytes, encoded_header_bytes);
        assert_eq!(decoded_vault_file.encrypted_contents, encrypted_contents);
    }

    #[test]
    fn unknown_version_is_reported() {
        let mut vault_file_bytes = sample_header().encode().expect("cabecera");
        vault_file_bytes[4] = 99;

        let decode_result = decode_vault_file(&vault_file_bytes);

        assert!(matches!(decode_result, Err(VaultError::UnsupportedVaultVersion { found_version: 99 })));
    }

    #[test]
    fn truncated_file_is_reported_without_panicking() {
        let vault_file_bytes = sample_header().encode().expect("cabecera");

        for truncated_length in 0..vault_file_bytes.len() {
            assert!(decode_vault_file(&vault_file_bytes[..truncated_length]).is_err());
        }
    }
}
