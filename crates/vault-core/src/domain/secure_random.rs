//! Aleatoriedad criptográfica del sistema operativo (`getrandom`).

use crate::domain::encryption_key::EncryptionKey;
use crate::domain::vault_error::{VaultError, VaultResult};

pub fn random_bytes<const BYTE_COUNT: usize>() -> VaultResult<[u8; BYTE_COUNT]> {
    let mut random_buffer = [0u8; BYTE_COUNT];
    getrandom::fill(&mut random_buffer).map_err(|_| VaultError::Cryptography)?;
    Ok(random_buffer)
}

pub fn random_encryption_key() -> VaultResult<EncryptionKey> {
    EncryptionKey::fill_in_place(|key_slot| getrandom::fill(key_slot).map_err(|_| VaultError::Cryptography))
}

/// Índice uniforme en `0..exclusive_upper_bound` sin sesgo de módulo (muestreo por rechazo).
pub fn random_index_below(exclusive_upper_bound: usize) -> VaultResult<usize> {
    if exclusive_upper_bound == 0 {
        return Err(VaultError::invalid_input("emptyRandomRange"));
    }

    let upper_bound = exclusive_upper_bound as u64;
    let rejection_threshold = u64::MAX - (u64::MAX % upper_bound);
    loop {
        let random_value = getrandom::u64().map_err(|_| VaultError::Cryptography)?;
        if random_value < rejection_threshold {
            #[allow(clippy::cast_possible_truncation)]
            return Ok((random_value % upper_bound) as usize);
        }
    }
}

/// Mezcla Fisher-Yates con aleatoriedad criptográfica.
pub fn shuffle_securely<T>(elements: &mut [T]) -> VaultResult<()> {
    for current_position in (1..elements.len()).rev() {
        let swap_position = random_index_below(current_position + 1)?;
        elements.swap(current_position, swap_position);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_index_stays_inside_bounds() {
        for _ in 0..1_000 {
            assert!(random_index_below(7).expect("aleatoriedad disponible") < 7);
        }
    }

    #[test]
    fn empty_range_is_rejected() {
        assert!(random_index_below(0).is_err());
    }
}
