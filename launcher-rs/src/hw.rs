//! Deteccao de hardware para registrar no log e avisar (nunca bloqueia).
//! RAM: `GlobalMemoryStatusEx`. VRAM dedicada: PowerShell (registro do driver, com
//! `Win32_VideoController` como alternativa), pois DXGI exigiria crates novas.
use crate::options::CACHE_LARGE_MB;

const GIB: u64 = 1 << 30;
pub const MIN_RAM_GIB: u64 = 8;
pub const MIN_VRAM_GIB: u64 = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Hardware {
    pub ram_total: Option<u64>,
    pub ram_free: Option<u64>,
    pub vram: Option<u64>,
}

#[cfg(windows)]
fn ram() -> Option<(u64, u64)> {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_ext: u64,
    }
    extern "system" {
        fn GlobalMemoryStatusEx(buf: *mut MemoryStatusEx) -> i32;
    }
    let mut m = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page: 0,
        avail_page: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_ext: 0,
    };
    // SAFETY: `m` tem o layout de MEMORYSTATUSEX e `length` preenchido.
    (unsafe { GlobalMemoryStatusEx(&mut m) } != 0).then_some((m.total_phys, m.avail_phys))
}

#[cfg(not(windows))]
fn ram() -> Option<(u64, u64)> {
    None
}

#[cfg(windows)]
fn vram() -> Option<u64> {
    const SCRIPT: &str = "$m=0;\
$k='HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}';\
Get-ChildItem $k -ErrorAction SilentlyContinue | ForEach-Object { $v=(Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue).'HardwareInformation.qwMemorySize'; if($v -gt $m){$m=[uint64]$v} };\
if($m -eq 0){ Get-CimInstance Win32_VideoController | ForEach-Object { if($_.AdapterRAM -gt $m){$m=[uint64]$_.AdapterRAM} } };\
Write-Output $m";
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    parse_vram(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(not(windows))]
fn vram() -> Option<u64> {
    None
}

fn parse_vram(text: &str) -> Option<u64> {
    text.lines().filter_map(|l| l.trim().parse::<u64>().ok()).max().filter(|&v| v > 0)
}

/// Le a RAM (barato) e, se `with_vram`, a VRAM (algumas centenas de ms via PowerShell).
pub fn detect(with_vram: bool) -> Hardware {
    let r = ram();
    Hardware { ram_total: r.map(|x| x.0), ram_free: r.map(|x| x.1), vram: if with_vram { vram() } else { None } }
}

fn gib(b: u64) -> String {
    format!("{:.1} GiB", b as f64 / GIB as f64)
}

impl Hardware {
    pub fn summary(&self) -> String {
        let f = |v: Option<u64>| v.map(gib).unwrap_or_else(|| "desconhecida".into());
        format!("RAM total {}, livre {}; VRAM dedicada {}", f(self.ram_total), f(self.ram_free), f(self.vram))
    }

    /// Avisos para escala > 2 ou cache grande (so avisa; valores desconhecidos nao geram aviso).
    pub fn warnings(&self, scale: Option<u32>, cache_mb: Option<u32>) -> Vec<String> {
        let mut w = Vec::new();
        let big_scale = scale.is_some_and(|s| s > 2);
        let big_cache = cache_mb.is_some_and(|c| c >= CACHE_LARGE_MB);
        if !big_scale && !big_cache {
            return w;
        }
        let what = match (big_scale, big_cache) {
            (true, true) => "escala interna alta e cache grande",
            (true, false) => "escala interna alta",
            _ => "cache grande",
        };
        if self.ram_total.is_some_and(|r| r < MIN_RAM_GIB * GIB) {
            w.push(format!("RAM abaixo de {MIN_RAM_GIB} GiB com {what}: pode faltar memoria"));
        }
        if big_scale && self.vram.is_some_and(|v| v < MIN_VRAM_GIB * GIB) {
            w.push(format!("VRAM abaixo de {MIN_VRAM_GIB} GiB com escala {}: pode ficar lento ou falhar", scale.unwrap_or(0)));
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warns_only_when_big_and_low() {
        let low = Hardware { ram_total: Some(4 * GIB), ram_free: None, vram: Some(2 * GIB) };
        assert!(low.warnings(Some(2), Some(0)).is_empty());
        assert_eq!(low.warnings(Some(4), None).len(), 2);
        assert_eq!(low.warnings(None, Some(2048)).len(), 1); // so RAM
        let ok = Hardware { ram_total: Some(16 * GIB), ram_free: None, vram: Some(8 * GIB) };
        assert!(ok.warnings(Some(8), Some(4096)).is_empty());
        assert!(Hardware::default().warnings(Some(8), Some(4096)).is_empty());
    }

    #[test]
    fn vram_parse_takes_max() {
        assert_eq!(parse_vram("1073741824\r\n8589934592\r\n"), Some(8 << 30));
        assert_eq!(parse_vram("0\n"), None);
    }
}
