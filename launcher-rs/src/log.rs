//! Log em arquivo (`<jogo>/logs/launcher_AAAA-MM-DD.log`) e no stderr, com data/hora UTC e nivel
//! (regra 6). Falhar ao gravar o log nunca derruba o launcher.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static FILE: Mutex<Option<File>> = Mutex::new(None);

/// Converte segundos Unix em (ano, mes, dia, h, m, s) UTC (algoritmo de Howard Hinnant).
pub fn civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = (secs % 86_400) as u32;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Carimbo `AAAAMMDD_HHMMSS` para nomes de arquivo.
pub fn stamp() -> String {
    let (y, mo, d, h, mi, s) = civil(now());
    format!("{y:04}{mo:02}{d:02}_{h:02}{mi:02}{s:02}")
}

pub fn init(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let (y, mo, d, ..) = civil(now());
    let path = dir.join(format!("launcher_{y:04}-{mo:02}-{d:02}.log"));
    if let Ok(f) = OpenOptions::new().create(true).append(true).open(path) {
        *FILE.lock().unwrap() = Some(f);
    }
}

pub fn write(level: &str, msg: &str) {
    let (y, mo, d, h, mi, s) = civil(now());
    let line = format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}Z {level} {msg}");
    eprintln!("{line}");
    if let Some(f) = FILE.lock().unwrap().as_mut() {
        let _ = writeln!(f, "{line}");
    }
}

#[macro_export]
macro_rules! info { ($($a:tt)*) => { $crate::log::write("INFO", &format!($($a)*)) }; }
#[macro_export]
macro_rules! warn { ($($a:tt)*) => { $crate::log::write("WARN", &format!($($a)*)) }; }
#[macro_export]
macro_rules! error { ($($a:tt)*) => { $crate::log::write("ERROR", &format!($($a)*)) }; }

#[cfg(test)]
mod tests {
    use super::civil;
    #[test]
    fn civil_known_dates() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(1_000_000_000), (2001, 9, 9, 1, 46, 40));
        assert_eq!(civil(1_709_210_096), (2024, 2, 29, 12, 34, 56));
    }
}
