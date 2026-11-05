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
}

/// Backend de desenho aceito em `--renderer` / `renderer =`. Devolve o valor canonico de
/// `PS2X_GS` (`cpu-serial` vira `cpu`; `hw-thread` vira `hw`).
pub fn validate_renderer(v: &str) -> Result<String> {
    let v = v.trim().to_ascii_lowercase();
    match v.as_str() {
        "cpu" | "cpu-serial" => Ok("cpu".into()),
        "hw" | "hw-thread" => Ok("hw".into()),
        "cpu-thread" | "cpu-bands" | "d3d11" | "hw-serial" => Ok(v),
        _ => Err(err!("renderizador invalido '{v}': use cpu, cpu-thread, hw (avancado: cpu-bands, d3d11, hw-serial)")),
    }
}

/// Renderizadores por hardware (aceitam `PS2X_GS_SCALE`).
pub fn is_hw(renderer: &str) -> bool {
    matches!(renderer, "hw" | "hw-serial" | "hw-thread")
}

/// Renderizadores com rasterizacao em faixas (aceitam `PS2X_GS_BANDS`; nunca junto com `hw`).
pub fn uses_bands(renderer: &str) -> bool {
    matches!(renderer, "cpu-thread" | "cpu-bands")
}

/// Faixas de rasterizacao (`PS2X_GS_BANDS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bands {
    Auto,
    Off,
    N(u32),
}

impl Bands {
    /// Texto do cfg/CLI.
    pub fn key(self) -> String {
        match self {
            Bands::Auto => "auto".into(),
            Bands::Off => "off".into(),
            Bands::N(n) => n.to_string(),
        }
    }

    /// Valor de `PS2X_GS_BANDS` (0 desliga).
    pub fn env(self) -> String {
        match self {
            Bands::Auto => "auto".into(),
            Bands::Off => "0".into(),
            Bands::N(n) => n.to_string(),
        }
    }
}

/// Modo de threads experimentais do runtime (`PS2X_THREADS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Threads {
    Off,
    Vu,
    VuVerify,
    Iop,
}

impl Threads {
    pub const ALL: [Threads; 4] = [Threads::Off, Threads::Vu, Threads::VuVerify, Threads::Iop];

    /// Texto do cfg/CLI e valor de `PS2X_THREADS`.
    pub fn key(self) -> &'static str {
        match self {
            Threads::Off => "off",
            Threads::Vu => "vu",
            Threads::VuVerify => "vu-verify",
            Threads::Iop => "iop",
        }
    }

    pub fn parse(v: &str) -> Option<Threads> {
        let v = v.trim().to_ascii_lowercase();
        Threads::ALL.into_iter().find(|t| t.key() == v)
    }
}

pub fn parse_threads(v: &str) -> Result<Threads> {
    Threads::parse(v).ok_or_else(|| err!("modo de threads invalido '{v}': use off, vu, vu-verify ou iop"))
}

/// Teto de threads de trabalho do renderer hw (`PS2X_GS_HW_WORKERS`); 0 desliga.
pub const HW_WORKERS_MAX: u32 = 64;

pub fn parse_hw_workers(v: &str) -> Result<u32> {
    match v.trim().parse::<u32>() {
        Ok(n) if n <= HW_WORKERS_MAX => Ok(n),
        _ => Err(err!("workers do hw invalidos '{v}': espera 0 (desliga) a {HW_WORKERS_MAX}")),
    }
}

pub const BANDS_MAX: u32 = 64;
pub const QUEUE_KB_RANGE: std::ops::RangeInclusive<u32> = 256..=262_144;

pub fn parse_bands(v: &str) -> Result<Bands> {
    match v.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(Bands::Auto),
        "off" | "0" | "nao" => Ok(Bands::Off),
        t => match t.parse::<u32>() {
            Ok(n) if (1..=BANDS_MAX).contains(&n) => Ok(Bands::N(n)),
            _ => Err(err!("faixas invalidas '{v}': use auto, off ou um inteiro de 1 a {BANDS_MAX}")),
        },
    }
}

pub fn parse_queue_kb(v: &str) -> Result<u32> {
    let n: u32 = v.trim().parse().map_err(|_| err!("fila do GS invalida '{v}': espera KiB inteiros"))?;
    if QUEUE_KB_RANGE.contains(&n) {
        Ok(n)
    } else {
        Err(err!("fila do GS {n} KiB fora da faixa {}..{}", QUEUE_KB_RANGE.start(), QUEUE_KB_RANGE.end()))
    }
}

/// Forca do CRT em percentual (0..100).
pub fn parse_crt_strength(v: &str) -> Result<u32> {
    match v.trim().trim_end_matches('%').parse::<u32>() {
        Ok(n) if n <= 100 => Ok(n),
        _ => Err(err!("forca do CRT invalida '{v}': espera 0 a 100 (percentual)")),
    }
}

/// `PS2X_POSTFX_CRT_STRENGTH` a partir do percentual: sempre fracao `0.00..1.00` (sem ambiguidade
/// com a regra "valor <= 1 = fracao" do runtime).
pub fn crt_strength_env(percent: u32) -> String {
    format!("{:.2}", percent as f64 / 100.0)
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
        for ok in ["cpu", "CPU-Thread", "d3d11", " hw ", "cpu-bands", "hw-serial"] {
            assert!(validate_renderer(ok).is_ok(), "{ok}");
        }
        assert_eq!(validate_renderer("HW").unwrap(), "hw");
        assert_eq!(validate_renderer("hw-thread").unwrap(), "hw");
        assert_eq!(validate_renderer("cpu-serial").unwrap(), "cpu");
        assert!(is_hw("hw-serial") && !is_hw("cpu-thread") && uses_bands("cpu-bands") && !uses_bands("hw"));
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
    fn threads_and_workers() {
        assert_eq!(parse_threads("VU-Verify").unwrap(), Threads::VuVerify);
        assert_eq!(Threads::Iop.key(), "iop");
        assert!(parse_threads("all").is_err());
        assert_eq!(parse_hw_workers("0").unwrap(), 0);
        assert_eq!(parse_hw_workers("4").unwrap(), 4);
        assert!(parse_hw_workers("65").is_err() && parse_hw_workers("-1").is_err() && parse_hw_workers("x").is_err());
    }

    #[test]
    fn crt_levels() {
        assert_eq!(parse_crt("Forte").unwrap(), Crt::Forte);
        assert!(parse_crt("max").is_err());
    }

    #[test]
    fn bands_queue_strength() {
        assert_eq!(parse_bands("AUTO").unwrap(), Bands::Auto);
        assert_eq!(parse_bands("off").unwrap(), Bands::Off);
        assert_eq!(parse_bands("0").unwrap(), Bands::Off);
        assert_eq!(parse_bands("8").unwrap().env(), "8");
        assert_eq!(Bands::Off.env(), "0");
        assert!(parse_bands("65").is_err() && parse_bands("x").is_err());
        assert_eq!(parse_queue_kb("8192").unwrap(), 8192);
        assert!(parse_queue_kb("1").is_err());
        assert_eq!(parse_crt_strength("50%").unwrap(), 50);
        assert!(parse_crt_strength("101").is_err());
        assert_eq!(crt_strength_env(100), "1.00");
        assert_eq!(crt_strength_env(35), "0.35");
    }
}
