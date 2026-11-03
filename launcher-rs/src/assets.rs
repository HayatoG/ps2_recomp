//! Extracao dos arquivos do disco para a pasta de trabalho (ignorada pelo git).
//!
//! - Copia em streaming para `*.part` e renomeia ao concluir (nunca deixa arquivo truncado
//!   passando por completo).
//! - Calcula o sha256 durante a copia e grava um manifesto (`.ps2launch-manifest.toml`).
//! - Confere espaco livre antes de comecar.
//! - Execucoes seguintes so copiam o que falta ou mudou de tamanho; `--force` refaz tudo.
//! - Arquivos ja presentes sem manifesto (extraidos pelo launcher antigo) sao adotados pelo tamanho;
//!   o hash deles e preenchido por `verify --deep`.
use crate::{config::GameConfig, dump::hex, err, error::Result, fsx, info, iso::{Entry, Iso}, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const MANIFEST: &str = ".ps2launch-manifest.toml";

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Manifest {
    #[serde(default, rename = "file")]
    pub files: Vec<FileRec>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileRec {
    pub path: String,
    pub size: u64,
    /// Vazio = adotado sem hash (ver `verify --deep`).
    #[serde(default)]
    pub sha256: String,
}

impl Manifest {
    pub fn load(work: &Path) -> Manifest {
        std::fs::read_to_string(work.join(MANIFEST))
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, work: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).map_err(|e| err!("manifesto: {e}"))?;
        let tmp = work.join(format!("{MANIFEST}.part"));
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, work.join(MANIFEST))?;
        Ok(())
    }

    fn get(&self, path: &str) -> Option<&FileRec> {
        self.files.iter().find(|f| f.path == path)
    }

    pub fn set(&mut self, rec: FileRec) {
        match self.files.iter_mut().find(|f| f.path == rec.path) {
            Some(slot) => *slot = rec,
            None => self.files.push(rec),
        }
    }
}

/// Expande os nomes pedidos em (caminho_relativo, entrada); diretorio vira seus arquivos.
pub fn plan(iso: &mut Iso, wanted: &[String]) -> Result<Vec<(String, Entry)>> {
    let mut out = Vec::new();
    for name in wanted {
        let e = iso.find(name)?.clone();
        if e.is_dir {
            for sub in iso.list_dir(name)? {
                if !sub.is_dir {
                    out.push((format!("{name}/{}", sub.name), sub));
                }
            }
        } else {
            out.push((name.clone(), e));
        }
    }
    Ok(out)
}

fn target(work: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(work.to_path_buf(), |p, part| p.join(part))
}

/// Andamento da extracao: bytes copiados/total e o arquivo atual.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
    pub file: String,
}

pub fn extract(cfg: &GameConfig, iso_path: &Path, force: bool) -> Result<()> {
    extract_with(cfg, iso_path, force, &mut |_| {})
}

/// [`extract`] avisando o andamento a cada bloco copiado (a GUI mostra a barra de progresso).
pub fn extract_with(cfg: &GameConfig, iso_path: &Path, force: bool, progress: &mut dyn FnMut(&Progress)) -> Result<()> {
    let mut iso = Iso::open(iso_path)?;
    let items = plan(&mut iso, &cfg.extract_files)?;
    std::fs::create_dir_all(&cfg.work_dir)?;
    let mut manifest = Manifest::load(&cfg.work_dir);

    let todo: Vec<&(String, Entry)> = items
        .iter()
        .filter(|(rel, e)| {
            if force {
                return true;
            }
            let ok = std::fs::metadata(target(&cfg.work_dir, rel)).map(|m| m.len() == e.size as u64).unwrap_or(false);
            if ok && manifest.get(rel).is_none() {
                manifest.set(FileRec { path: rel.clone(), size: e.size as u64, sha256: String::new() });
            }
            !ok
        })
        .collect();

    if todo.is_empty() {
        info!("{} ja contem todos os arquivos ({}); nada a extrair (use --force)", cfg.work_dir.display(), items.len());
        manifest.save(&cfg.work_dir)?;
        return Ok(());
    }

    let need: u64 = todo.iter().map(|(_, e)| e.size as u64).sum();
    if let Some(free) = fsx::free_space(&cfg.work_dir) {
        if free < need + (64 << 20) {
            return Err(err!(
                "espaco insuficiente em {}: preciso de {} MiB e ha {} MiB livres",
                cfg.work_dir.display(),
                need >> 20,
                free >> 20
            ));
        }
    } else {
        warn!("nao consegui medir o espaco livre; seguindo sem checagem");
    }

    info!("extraindo {} arquivo(s), {} MiB, para {}", todo.len(), need >> 20, cfg.work_dir.display());
    let mut done = 0u64;
    let mut state = Progress { done: 0, total: need, file: String::new() };
    for (rel, entry) in todo {
        let dest = target(&cfg.work_dir, rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let part = dest.with_file_name(format!("{}.part", dest.file_name().unwrap().to_string_lossy()));
        let mut hasher = Sha256::new();
        state.file = rel.clone();
        progress(&state);
        {
            let mut w = BufWriter::with_capacity(1 << 20, File::create(&part)?);
            iso.copy_to(entry, &mut |chunk| {
                hasher.update(chunk);
                w.write_all(chunk)?;
                state.done += chunk.len() as u64;
                progress(&state);
                Ok(())
            })?;
            w.flush()?;
        }
        std::fs::rename(&part, &dest)?;
        let rec = FileRec { path: rel.clone(), size: entry.size as u64, sha256: hex(&hasher.finalize()) };
        done += rec.size;
        info!("extraido {rel} ({} bytes) sha256={} [{}/{} MiB]", rec.size, rec.sha256, done >> 20, need >> 20);
        manifest.set(rec);
        manifest.save(&cfg.work_dir)?; // salva a cada arquivo: permite retomar
    }
    Ok(())
}

/// Estado dos dados extraidos, pelo tamanho (sem ler o conteudo).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocalStatus {
    pub present: usize,
    pub total: usize,
    pub bytes_present: u64,
    pub bytes_total: u64,
    /// Espaco livre no volume da pasta de trabalho (ou do jogo, se ela nao existir).
    pub free: Option<u64>,
}

impl LocalStatus {
    pub fn complete(&self) -> bool {
        self.total > 0 && self.present == self.total
    }
}

/// Confere `[[dump.file]]` (ou, sem a tabela, o manifesto) contra a pasta de trabalho.
pub fn local_status(cfg: &GameConfig) -> LocalStatus {
    let known: Vec<(String, u64)> = if cfg.dump_files.is_empty() {
        Manifest::load(&cfg.work_dir).files.into_iter().map(|f| (f.path, f.size)).collect()
    } else {
        cfg.dump_files.iter().map(|f| (f.name.clone(), f.size)).collect()
    };
    let mut st = LocalStatus { total: known.len(), ..Default::default() };
    for (rel, size) in known {
        st.bytes_total += size;
        if std::fs::metadata(target(&cfg.work_dir, &rel)).map(|m| m.len() == size).unwrap_or(false) {
            st.present += 1;
            st.bytes_present += size;
        }
    }
    let probe = if cfg.work_dir.is_dir() { cfg.work_dir.as_path() } else { cfg.root.as_path() };
    st.free = fsx::free_space(probe);
    st
}

/// `verify --deep`: rehash de tudo que esta no manifesto; preenche hashes adotados.
pub fn verify_deep(cfg: &GameConfig) -> Result<()> {
    let mut manifest = Manifest::load(&cfg.work_dir);
    if manifest.files.is_empty() {
        return Err(err!("sem manifesto em {}; rode 'extract' primeiro", cfg.work_dir.display()));
    }
    let mut bad = 0;
    for rec in manifest.files.clone() {
        let path = target(&cfg.work_dir, &rec.path);
        let mut f = match File::open(&path) {
            Ok(f) => f,
            Err(_) => {
                warn!("FALTA {}", rec.path);
                bad += 1;
                continue;
            }
        };
        let (mut h, mut buf, mut len) = (Sha256::new(), vec![0u8; 1 << 20], 0u64);
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            len += n as u64;
        }
        let digest = hex(&h.finalize());
        if len != rec.size || (!rec.sha256.is_empty() && digest != rec.sha256) {
            warn!("DIFERENTE {} (tamanho {len}, esperado {})", rec.path, rec.size);
            bad += 1;
        } else {
            if rec.sha256.is_empty() {
                manifest.set(FileRec { sha256: digest.clone(), ..rec.clone() });
            }
            info!("ok {} sha256={digest}", rec.path);
        }
    }
    manifest.save(&cfg.work_dir)?;
    if bad > 0 {
        return Err(err!("{bad} arquivo(s) com problema; rode 'extract --force'"));
    }
    Ok(())
}

/// `deep` compara tambem o sha256 dos arquivos extraidos com `[[dump.file]]` do game.toml.
pub fn check_known_hashes(cfg: &GameConfig) -> Result<()> {
    let manifest = Manifest::load(&cfg.work_dir);
    let mut bad = 0;
    for known in cfg.dump_files.iter().filter(|k| !k.sha256.is_empty()) {
        match manifest.get(&known.name) {
            Some(rec) if rec.sha256.is_empty() => warn!("{}: sem hash no manifesto (rode verify --deep)", known.name),
            Some(rec) if rec.sha256 == known.sha256 => {}
            Some(_) => {
                warn!("{}: sha256 diferente da edicao conhecida", known.name);
                bad += 1;
            }
            None => {
                warn!("{}: nao extraido", known.name);
                bad += 1;
            }
        }
    }
    if bad > 0 {
        return Err(err!("{bad} arquivo(s) nao batem com a edicao conhecida"));
    }
    Ok(())
}

/// Imprime o bloco `[[dump.file]]` a partir do manifesto (para colar no game.toml de uma edicao nova).
pub fn print_known_table(cfg: &GameConfig) -> Result<()> {
    let manifest = Manifest::load(&cfg.work_dir);
    if manifest.files.iter().any(|f| f.sha256.is_empty()) {
        return Err(err!("ha arquivos sem hash no manifesto; rode 'verify --deep' antes"));
    }
    for f in &manifest.files {
        println!("[[dump.file]]
name = \"{}\"
size = {}
sha256 = \"{}\"
", f.path, f.size, f.sha256);
    }
    Ok(())
}
