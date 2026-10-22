//! Códigos TOTP (RFC 6238) a partir de un secreto base32 o de una URI `otpauth://`.

use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use url::Url;
use zeroize::Zeroizing;

use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::{VaultError, VaultResult};

pub const DEFAULT_DIGIT_COUNT: u32 = 6;
pub const DEFAULT_PERIOD_SECONDS: u64 = 30;
pub const SUPPORTED_DIGIT_COUNTS: std::ops::RangeInclusive<u32> = DEFAULT_DIGIT_COUNT..=8;
const OTPAUTH_SCHEME: &str = "otpauth://";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotpHashAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl TotpHashAlgorithm {
    fn from_uri_name(algorithm_name: &str) -> VaultResult<Self> {
        match algorithm_name.to_ascii_uppercase().as_str() {
            "SHA1" => Ok(Self::Sha1),
            "SHA256" => Ok(Self::Sha256),
            "SHA512" => Ok(Self::Sha512),
            _ => Err(VaultError::InvalidTotpSecret),
        }
    }

    fn compute_hmac(self, secret_key: &[u8], message: &[u8]) -> VaultResult<Vec<u8>> {
        match self {
            Self::Sha1 => compute_hmac_with::<Hmac<Sha1>>(secret_key, message),
            Self::Sha256 => compute_hmac_with::<Hmac<Sha256>>(secret_key, message),
            Self::Sha512 => compute_hmac_with::<Hmac<Sha512>>(secret_key, message),
        }
    }
}

fn compute_hmac_with<MacAlgorithm: Mac + KeyInit>(secret_key: &[u8], message: &[u8]) -> VaultResult<Vec<u8>> {
    let mut message_authenticator =
        <MacAlgorithm as KeyInit>::new_from_slice(secret_key).map_err(|_| VaultError::InvalidTotpSecret)?;
    message_authenticator.update(message);
    Ok(message_authenticator.finalize().into_bytes().to_vec())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpCode {
    pub current_code: String,
    pub seconds_until_refresh: u64,
    pub period_seconds: u64,
}

pub struct TotpConfiguration {
    secret_key_bytes: Zeroizing<Vec<u8>>,
    hash_algorithm: TotpHashAlgorithm,
    digit_count: u32,
    period_seconds: u64,
}

impl TotpConfiguration {
    pub fn parse(totp_secret: &SecretText) -> VaultResult<Self> {
        let trimmed_secret = totp_secret.expose_secret().trim();
        // Sin `to_ascii_lowercase()` del secreto entero: dejaría otra copia sin borrar.
        if trimmed_secret.get(..OTPAUTH_SCHEME.len()).is_some_and(|scheme| scheme.eq_ignore_ascii_case(OTPAUTH_SCHEME)) {
            return Self::parse_otpauth_uri(trimmed_secret);
        }
        Ok(Self {
            secret_key_bytes: decode_base32_secret(trimmed_secret)?,
            hash_algorithm: TotpHashAlgorithm::Sha1,
            digit_count: DEFAULT_DIGIT_COUNT,
            period_seconds: DEFAULT_PERIOD_SECONDS,
        })
    }

    fn parse_otpauth_uri(otpauth_uri: &str) -> VaultResult<Self> {
        let parsed_uri = Url::parse(otpauth_uri).map_err(|_| VaultError::InvalidTotpSecret)?;
        let find_query_parameter = |parameter_name: &str| {
            parsed_uri
                .query_pairs()
                .find(|(query_key, _)| query_key.eq_ignore_ascii_case(parameter_name))
                .map(|(_, query_value)| Zeroizing::new(query_value.into_owned()))
        };

        // `Url` guarda su propia copia de la URI, que no se puede borrar: límite conocido.
        let base32_secret = find_query_parameter("secret").ok_or(VaultError::InvalidTotpSecret)?;
        let hash_algorithm = find_query_parameter("algorithm")
            .map_or(Ok(TotpHashAlgorithm::Sha1), |algorithm_name| TotpHashAlgorithm::from_uri_name(&algorithm_name))?;
        let digit_count = find_query_parameter("digits")
            .map_or(Ok(DEFAULT_DIGIT_COUNT), |digits_text| digits_text.parse::<u32>())
            .map_err(|_| VaultError::InvalidTotpSecret)?;
        let period_seconds = find_query_parameter("period")
            .map_or(Ok(DEFAULT_PERIOD_SECONDS), |period_text| period_text.parse::<u64>())
            .map_err(|_| VaultError::InvalidTotpSecret)?;

        if !SUPPORTED_DIGIT_COUNTS.contains(&digit_count) || period_seconds == 0 {
            return Err(VaultError::InvalidTotpSecret);
        }
        Ok(Self { secret_key_bytes: decode_base32_secret(&base32_secret)?, hash_algorithm, digit_count, period_seconds })
    }

    pub fn code_at(&self, unix_timestamp_seconds: u64) -> VaultResult<TotpCode> {
        let time_step_counter = unix_timestamp_seconds / self.period_seconds;
        let hmac_digest = self.hash_algorithm.compute_hmac(&self.secret_key_bytes, &time_step_counter.to_be_bytes())?;

        // Truncado dinámico (RFC 4226 §5.3).
        let truncation_offset = usize::from(hmac_digest[hmac_digest.len() - 1] & 0x0f);
        let truncated_value = u32::from_be_bytes([
            hmac_digest[truncation_offset] & 0x7f,
            hmac_digest[truncation_offset + 1],
            hmac_digest[truncation_offset + 2],
            hmac_digest[truncation_offset + 3],
        ]);
        let one_time_code = truncated_value % 10u32.pow(self.digit_count);

        Ok(TotpCode {
            current_code: format!("{one_time_code:0width$}", width = self.digit_count as usize),
            seconds_until_refresh: self.period_seconds - (unix_timestamp_seconds % self.period_seconds),
            period_seconds: self.period_seconds,
        })
    }
}

fn decode_base32_secret(base32_secret: &str) -> VaultResult<Zeroizing<Vec<u8>>> {
    // Mismo tamaño que la entrada: reservar evita realocar (y dejar copias sin borrar).
    let mut normalized_secret = Zeroizing::new(String::with_capacity(base32_secret.len()));
    normalized_secret.extend(
        base32_secret
            .chars()
            .filter(|secret_character| !secret_character.is_whitespace() && *secret_character != '=')
            .map(|secret_character| secret_character.to_ascii_uppercase()),
    );
    if normalized_secret.is_empty() {
        return Err(VaultError::InvalidTotpSecret);
    }
    base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &normalized_secret)
        .filter(|decoded_bytes| !decoded_bytes.is_empty())
        .map(Zeroizing::new)
        .ok_or(VaultError::InvalidTotpSecret)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rfc6238_configuration(ascii_secret: &str, hash_algorithm: TotpHashAlgorithm) -> TotpConfiguration {
        TotpConfiguration {
            secret_key_bytes: Zeroizing::new(ascii_secret.as_bytes().to_vec()),
            hash_algorithm,
            digit_count: 8,
            period_seconds: 30,
        }
    }

    #[test]
    fn matches_rfc6238_test_vectors() {
        let sha1_configuration = rfc6238_configuration("12345678901234567890", TotpHashAlgorithm::Sha1);
        let sha256_configuration =
            rfc6238_configuration("12345678901234567890123456789012", TotpHashAlgorithm::Sha256);
        let sha512_configuration = rfc6238_configuration(
            "1234567890123456789012345678901234567890123456789012345678901234",
            TotpHashAlgorithm::Sha512,
        );

        assert_eq!(sha1_configuration.code_at(59).expect("código").current_code, "94287082");
        assert_eq!(sha1_configuration.code_at(1_111_111_109).expect("código").current_code, "07081804");
        assert_eq!(sha256_configuration.code_at(59).expect("código").current_code, "46119246");
        assert_eq!(sha512_configuration.code_at(59).expect("código").current_code, "90693936");
    }

    #[test]
    fn parses_otpauth_uri_with_custom_parameters() {
        let otpauth_uri = SecretText::new(
            "otpauth://totp/Ejemplo:ana@correo.es?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&algorithm=SHA1&digits=8&period=30",
        );

        let totp_configuration = TotpConfiguration::parse(&otpauth_uri).expect("uri válida");

        assert_eq!(totp_configuration.code_at(59).expect("código").current_code, "94287082");
    }

    #[test]
    fn plain_base32_secret_with_spaces_is_accepted() {
        let spaced_secret = SecretText::new("gezd gnbv gy3t qojq gezd gnbv gy3t qojq");

        let totp_code = TotpConfiguration::parse(&spaced_secret).expect("secreto").code_at(59).expect("código");

        assert_eq!(totp_code.current_code, "287082");
        assert_eq!(totp_code.seconds_until_refresh, 1);
    }

    #[test]
    fn invalid_secret_is_rejected() {
        assert!(TotpConfiguration::parse(&SecretText::new("¡no es base32!")).is_err());
    }
}
