//! Localiza e valida o dump original do usuario (regra 4): serial (BOOT2) + sha256 do ELF de boot.
//! Edicao desconhecida e RECUSADA, mostrando o hash para o usuario reportar; nunca se adivinha.
use crate::{assets, config::GameConfig, err, error::Result, info, iso::Iso};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

/// Procura `iso_glob` (padrao `*.iso`; suporta so `*` no inicio) na pasta do jogo.
pub fn find_iso(cfg: &GameConfig, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return if p.is_file() { Ok(p.to_path_buf()) } else { Err(err!("imagem nao encontrada: {}", p.display())) };
    }
    let suffix = cfg.iso_glob.trim_start_matches('*').to_lowercase();
    let mut found: Vec<PathBuf> = std::fs::read_dir(&cfg.root)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.file_name().map(|n| n.to_string_lossy().to_lowercase().ends_with(&suffix)).unwrap_or(false)
        })
        .collect();
    found.sort();
    match found.len() {
        0 => Err(err!(
            "dump original nao encontrado em {} ({}). Coloque a sua imagem do jogo ali ou use --iso.",
            cfg.root.display(),
            cfg.iso_glob
        )),
        1 => Ok(found.remove(0)),
        _ => Err(err!("mais de uma imagem em {}; use --iso. Candidatas: {:?}", cfg.root.display(), found)),
    }
}

/// Confere serial e hash do ELF de boot contra o `game.toml`.
pub fn verify(cfg: &GameConfig, iso_path: &Path) -> Result<()> {
    let mut iso = Iso::open(iso_path)?;
    let (boot, _) = iso.boot_info()?;
    if boot != cfg.serial {
        return Err(err!(
            "Serial do disco ({boot}) difere do esperado ({}). Esta edicao nao e suportada por este port.",
            cfg.serial
        ));
    }
    let entry = iso.find(&boot)?.clone();
    let digest = sha256_hex(&iso.read(&entry)?);
    if digest != cfg.boot_elf_sha256 {
        return Err(err!(
            "Hash do {} nao confere.\n  obtido:   {digest}\n  esperado: {}\n\
             Dump diferente (outra edicao/revisao) ou corrompido. Nada sera executado.",
            cfg.serial,
            cfg.boot_elf_sha256
        ));
    }
    info!("dump verificado: {} serial={} sha256={digest}", iso_path.display(), cfg.serial);
    Ok(())
}

/// Confere, pelo diretorio da imagem (sem ler os dados), o tamanho de cada arquivo conhecido do
/// `[[dump.file]]`. Pega imagens truncadas/de outra edicao rapido; o hash fica para `verify --deep`.
pub fn verify_sizes(cfg: &GameConfig, iso_path: &Path) -> Result<()> {
    if cfg.dump_files.is_empty() {
        return Ok(());
    }
    let mut iso = Iso::open(iso_path)?;
    let items = assets::plan(&mut iso, &cfg.extract_files)?;
    let mut bad = Vec::new();
    for known in &cfg.dump_files {
        match items.iter().find(|(rel, _)| *rel == known.name) {
            Some((_, e)) if e.size as u64 == known.size => {}
            Some((_, e)) => bad.push(format!("{}: tamanho {} (esperado {})", known.name, e.size, known.size)),
            None => bad.push(format!("{}: ausente na imagem", known.name)),
        }
    }
    if !bad.is_empty() {
        return Err(err!("a imagem difere da edicao conhecida:
  {}
Dump de outra edicao ou truncado. Nada sera executado.", bad.join("
  ")));
    }
    info!("tamanhos conferidos: {} arquivos conhecidos", cfg.dump_files.len());
    Ok(())
}
