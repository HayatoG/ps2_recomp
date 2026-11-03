//! Leitura do `game.toml` (mesmo formato do launcher Python). Nada aqui e conteudo do jogo:
//! so identificacao e hashes para validar o dump do usuario.
use crate::{err, error::Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Raw {
    game: RawGame,
    dump: RawDump,
    #[serde(default)]
    extract: RawExtract,
    #[serde(default)]
    run: RawRun,
    #[serde(default)]
    art: RawArt,
}

/// `[art]`: so como a janela mostra a arte local do usuario (`<jogo>/artwork/`).
#[derive(Deserialize, Default)]
struct RawArt {
    /// Ponto da capa (x, y em 0..1) mantido no centro da faixa esquerda.
    focus: Option<[f32; 2]>,
}

#[derive(Deserialize)]
struct RawGame {
    name: String,
    serial: String,
    #[serde(default)]
    region: String,
}

#[derive(Deserialize)]
struct RawDump {
    #[serde(default = "default_glob")]
    iso_glob: String,
    boot_elf_sha256: String,
    /// Tabela de arquivos conhecidos da edicao (`[[dump.file]]`): nome, tamanho e sha256 opcional.
    #[serde(default)]
    file: Vec<DumpFile>,
}

/// Arquivo conhecido do disco (conhecimento nosso: so nome, tamanho e hash; nunca bytes do jogo).
#[derive(Deserialize, Debug, Clone)]
pub struct DumpFile {
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub sha256: String,
}

#[derive(Deserialize, Default)]
struct RawExtract {
    work_dir: Option<String>,
    files: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
struct RawRun {
    executable: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
}

fn default_glob() -> String {
    "*.iso".into()
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub root: PathBuf,
    pub name: String,
    pub serial: String,
    pub region: String,
    pub iso_glob: String,
    pub boot_elf_sha256: String,
    pub dump_files: Vec<DumpFile>,
    pub work_dir: PathBuf,
    pub extract_files: Vec<String>,
    pub run_executable: Option<PathBuf>,
    pub run_args: Vec<String>,
    pub run_env: BTreeMap<String, String>,
    /// Foco do corte da arte na janela (padrao: centro, um pouco acima).
    pub art_focus: [f32; 2],
}

impl GameConfig {
    pub fn load(game_dir: &Path) -> Result<Self> {
        let path = game_dir.join("game.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| err!("nao consegui ler {}: {e}", path.display()))?;
        // O toml rejeita BOM; editores do Windows costumam gravar um.
        let text = text.trim_start_matches('\u{feff}');
        let raw: Raw = toml::from_str(text).map_err(|e| err!("{} invalido: {e}", path.display()))?;
        let serial = raw.game.serial;
        Ok(GameConfig {
            root: game_dir.to_path_buf(),
            name: raw.game.name,
            region: raw.game.region,
            iso_glob: raw.dump.iso_glob,
            boot_elf_sha256: raw.dump.boot_elf_sha256.to_lowercase(),
            dump_files: raw.dump.file,
            work_dir: game_dir.join(raw.extract.work_dir.unwrap_or_else(|| "work".into())),
            extract_files: raw
                .extract
                .files
                .unwrap_or_else(|| vec![serial.clone(), "SYSTEM.CNF".into()]),
            run_executable: raw.run.executable.map(|e| game_dir.join(e)),
            run_args: raw.run.args,
            run_env: raw.run.env,
            art_focus: raw.art.focus.map(|[x, y]| [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)]).unwrap_or([0.5, 0.45]),
            serial,
        })
    }

    /// Frequencia de video do guest por regiao (PAL = 50 Hz; o runtime assume 60 por padrao).
    pub fn video_hz(&self) -> &'static str {
        if self.region.eq_ignore_ascii_case("PAL") {
            "50"
        } else {
            "60"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config_with_bom() {
        let dir = std::env::temp_dir().join("ps2launch_cfg_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("game.toml"),
            "\u{feff}[game]\nname=\"X\"\nserial=\"SLES_000.00\"\nregion=\"PAL\"\n[dump]\nboot_elf_sha256=\"AB\"\n",
        )
        .unwrap();
        let c = GameConfig::load(&dir).unwrap();
        assert_eq!(c.serial, "SLES_000.00");
        assert_eq!(c.boot_elf_sha256, "ab");
        assert_eq!(c.video_hz(), "50");
        assert_eq!(c.extract_files, vec!["SLES_000.00", "SYSTEM.CNF"]);
    }
}
