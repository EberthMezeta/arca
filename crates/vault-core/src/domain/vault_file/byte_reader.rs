use crate::domain::vault_error::{VaultError, VaultResult};

/// Lector secuencial de bytes que devuelve error (en vez de `panic`) si el archivo está truncado.
pub struct ByteReader<'source> {
    source_bytes: &'source [u8],
    read_position: usize,
}

impl<'source> ByteReader<'source> {
    pub fn new(source_bytes: &'source [u8]) -> Self {
        Self { source_bytes, read_position: 0 }
    }

    pub fn read_position(&self) -> usize {
        self.read_position
    }

    pub fn read_slice(&mut self, byte_count: usize) -> VaultResult<&'source [u8]> {
        let end_position = self
            .read_position
            .checked_add(byte_count)
            .filter(|end_position| *end_position <= self.source_bytes.len())
            .ok_or_else(|| VaultError::corrupted_file("archivo truncado"))?;
        let requested_slice = &self.source_bytes[self.read_position..end_position];
        self.read_position = end_position;
        Ok(requested_slice)
    }

    pub fn read_array<const BYTE_COUNT: usize>(&mut self) -> VaultResult<[u8; BYTE_COUNT]> {
        let requested_slice = self.read_slice(BYTE_COUNT)?;
        requested_slice.try_into().map_err(|_| VaultError::corrupted_file("archivo truncado"))
    }

    pub fn read_u8(&mut self) -> VaultResult<u8> {
        Ok(self.read_array::<1>()?[0])
    }

    pub fn read_u16_le(&mut self) -> VaultResult<u16> {
        self.read_array::<2>().map(u16::from_le_bytes)
    }

    pub fn read_u32_le(&mut self) -> VaultResult<u32> {
        self.read_array::<4>().map(u32::from_le_bytes)
    }

    pub fn read_remaining(&mut self) -> &'source [u8] {
        let remaining_slice = &self.source_bytes[self.read_position..];
        self.read_position = self.source_bytes.len();
        remaining_slice
    }
}
