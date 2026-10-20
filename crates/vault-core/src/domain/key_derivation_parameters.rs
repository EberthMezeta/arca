use serde::{Deserialize, Serialize};

/// Coste de Argon2id. Se guarda en claro en la cabecera del archivo
/// (y se autentica como AAD) para poder ajustarlo sin romper bóvedas antiguas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyDerivationParameters {
    pub memory_cost_kibibytes: u32,
    pub iteration_count: u32,
    pub parallelism_degree: u32,
}

impl KeyDerivationParameters {
    /// 256 MiB, 3 pasadas, 4 hilos: ~0,5-1 s en un portátil moderno.
    pub const RECOMMENDED: Self = Self {
        memory_cost_kibibytes: 256 * 1024,
        iteration_count: 3,
        parallelism_degree: 4,
    };

    /// Límite inferior aceptado al leer una bóveda (evita degradar la seguridad).
    pub const MINIMUM_ACCEPTED: Self = Self {
        memory_cost_kibibytes: 64 * 1024,
        iteration_count: 2,
        parallelism_degree: 1,
    };

    /// Límite superior: un archivo manipulado (bóveda o `.rvexport`) no puede forzar a reservar
    /// más de 1 GiB, que un equipo con poca RAM no aguantaría (4× el valor recomendado).
    pub const MAXIMUM_ACCEPTED: Self = Self {
        memory_cost_kibibytes: 1024 * 1024,
        iteration_count: 64,
        parallelism_degree: 16,
    };

    pub fn is_within(&self, lower_bound: &Self, upper_bound: &Self) -> bool {
        (lower_bound.memory_cost_kibibytes..=upper_bound.memory_cost_kibibytes).contains(&self.memory_cost_kibibytes)
            && (lower_bound.iteration_count..=upper_bound.iteration_count).contains(&self.iteration_count)
            && (lower_bound.parallelism_degree..=upper_bound.parallelism_degree).contains(&self.parallelism_degree)
    }
}
