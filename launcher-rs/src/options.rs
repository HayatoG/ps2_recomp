//! Validacao das opcoes de execucao (CLI e `launcher.cfg` usam as mesmas funcoes).
use crate::{err, error::Result};

/// Faixa da resolucao interna do renderer por hardware (`PS2X_GS_SCALE`).
pub const SCALE_RANGE: std::ops::RangeInclusive<u32> = 1..=8;
/// Teto do cache do VFS em MiB (`PS2X_VFS_CACHE_MB`); 0 desliga.
pub const CACHE_MAX_MB: u32 = 16_384;
/// A partir deste valor (MiB) o cache conta como "grande" para os avisos de hardware.
pub const CACHE_LARGE_MB: u32 = 1_024;

/// Intensidade do filtro CRT (pos-processamento).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crt {
    Off,
    Suave,
    Medio,
    Forte,
}

impl Crt {
    pub const ALL: [Crt; 4] = [Crt::Off, Crt::Suave, Crt::Medio, Crt::Forte];

    pub fn key(self) -> &'static str {
        match self {
            Crt::Off => "off",
            Crt::Suave => "suave",
            Crt::Medio => "medio",
            Crt::Forte => "forte",
        }
    }

    pub fn parse(v: &str) -> Option<Crt> {
        let v = v.to_ascii_lowercase();
        Crt::ALL.into_iter().find(|c| c.key() == v)
    }

    /// Valor de `PS2X_POSTFX_CRT_STRENGTH` (0..1). Tabela unica: ajustar aqui se o runtime
    /// pedir outra escala.
    pub fn strength(self) -> Option<&'static str> {
        match self {
            Crt::Off => None,
            Crt::Suave => Some("0.35"),
            Crt::Medio => Some("0.65"),
            Crt::Forte => Some("1.0"),
        }
    }
}

/// Backend de desenho aceito em `--renderer` / `renderer =` (valor de `PS2X_GS`).
pub fn validate_renderer(v: &str) -> Result<String> {
    let v = v.trim().to_ascii_lowercase();
    match v.as_str() {
        "cpu" | "cpu-thread" | "d3d11" | "hw" => Ok(v),
        _ => Err(err!("renderizador invalido '{v}': use cpu, cpu-thread, d3d11 ou hw")),
    }
}

pub fn parse_scale(v: &str) -> Result<u32> {
    let n: u32 = v.trim().parse().map_err(|_| err!("escala interna invalida '{v}': espera inteiro de 1 a 8"))?;
    if SCALE_RANGE.contains(&n) {
        Ok(n)
    } else {
        Err(err!("escala interna {n} fora da faixa 1..8"))
    }
}

pub fn parse_cache_mb(v: &str) -> Result<u32> {
    let n: u32 = v.trim().parse().map_err(|_| err!("cache do VFS invalido '{v}': espera MiB inteiros (0 desliga)"))?;
    if n <= CACHE_MAX_MB {
        Ok(n)
    } else {
        Err(err!("cache do VFS {n} MiB acima do maximo de {CACHE_MAX_MB}"))
    }
}

pub fn parse_crt(v: &str) -> Result<Crt> {
    Crt::parse(v.trim()).ok_or_else(|| err!("nivel de CRT invalido '{v}': use off, suave, medio ou forte"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_validation() {
        for ok in ["cpu", "CPU-Thread", "d3d11", " hw "] {
            assert!(validate_renderer(ok).is_ok(), "{ok}");
        }
        assert_eq!(validate_renderer("HW").unwrap(), "hw");
        let e = validate_renderer("vulkan").unwrap_err().to_string();
        assert!(e.contains("cpu-thread") && e.contains("hw"));
    }

    #[test]
    fn scale_and_cache_ranges() {
        assert_eq!(parse_scale("1").unwrap(), 1);
        assert_eq!(parse_scale("8").unwrap(), 8);
        assert!(parse_scale("0").is_err() && parse_scale("9").is_err() && parse_scale("x").is_err());
        assert_eq!(parse_cache_mb("0").unwrap(), 0);
        assert!(parse_cache_mb("16385").is_err() && parse_cache_mb("-1").is_err());
    }

    #[test]
    fn crt_levels() {
        assert_eq!(parse_crt("Forte").unwrap(), Crt::Forte);
        assert!(parse_crt("max").is_err());
        assert_eq!(Crt::Off.strength(), None);
    }
}
