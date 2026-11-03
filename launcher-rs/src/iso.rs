//! Leitor somente-leitura de ISO9660 de PS2. Detecta o layout do setor (2048 "cooked" ou 2352
//! bruto, modos 1 e 2) para aceitar tambem imagens BIN.
use crate::{err, error::Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const USER: u64 = 2048;

#[derive(Clone, Copy, Debug)]
struct Layout {
    sector: u64,
    offset: u64, // inicio dos 2048 bytes de usuario dentro do setor
}

const LAYOUTS: [Layout; 3] = [
    Layout { sector: 2048, offset: 0 },
    Layout { sector: 2352, offset: 16 }, // modo 1
    Layout { sector: 2352, offset: 24 }, // modo 2 form 1
];

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub lba: u32,
    pub size: u32,
    pub is_dir: bool,
}

pub struct Iso {
    f: File,
    layout: Layout,
    root: Vec<Entry>,
}

impl Iso {
    pub fn open(path: &Path) -> Result<Self> {
        let mut f = File::open(path).map_err(|e| err!("nao abri {}: {e}", path.display()))?;
        for layout in LAYOUTS {
            let mut pvd = [0u8; 2048];
            if read_user(&mut f, layout, 16, &mut pvd).is_ok() && &pvd[1..6] == b"CD001" {
                let lba = u32::from_le_bytes(pvd[158..162].try_into().unwrap());
                let size = u32::from_le_bytes(pvd[166..170].try_into().unwrap());
                let root = read_dir(&mut f, layout, lba, size)?;
                return Ok(Iso { f, layout, root });
            }
        }
        Err(err!("{} nao parece uma imagem ISO9660 (2048/2352)", path.display()))
    }

    pub fn find(&self, name: &str) -> Result<&Entry> {
        self.root
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| err!("{name} nao encontrado na raiz da imagem"))
    }

    pub fn list_dir(&mut self, name: &str) -> Result<Vec<Entry>> {
        let e = self.find(name)?.clone();
        if !e.is_dir {
            return Err(err!("{name} nao e um diretorio"));
        }
        read_dir(&mut self.f, self.layout, e.lba, e.size)
    }

    /// Le o arquivo inteiro na memoria (para arquivos pequenos: SYSTEM.CNF, ELF).
    pub fn read(&mut self, e: &Entry) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(e.size as usize);
        self.copy_to(e, &mut |chunk| {
            out.extend_from_slice(chunk);
            Ok(())
        })?;
        Ok(out)
    }

    /// Le em blocos (1 MiB) e entrega cada pedaco; usado para hash e copia em streaming.
    pub fn copy_to(&mut self, e: &Entry, sink: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
        const CHUNK_SECTORS: u64 = 512;
        let total_sectors = (e.size as u64).div_ceil(USER);
        let mut remaining = e.size as u64;
        let mut sec = 0u64;
        let mut buf = vec![0u8; (CHUNK_SECTORS * USER) as usize];
        while sec < total_sectors {
            let n = CHUNK_SECTORS.min(total_sectors - sec);
            let slice = &mut buf[..(n * USER) as usize];
            read_user(&mut self.f, self.layout, e.lba as u64 + sec, slice)?;
            let take = remaining.min(n * USER) as usize;
            sink(&slice[..take])?;
            remaining -= take as u64;
            sec += n;
        }
        Ok(())
    }

    /// Nome do ELF de BOOT2 e texto do SYSTEM.CNF.
    pub fn boot_info(&mut self) -> Result<(String, String)> {
        let e = self.find("SYSTEM.CNF")?.clone();
        let text = String::from_utf8_lossy(&self.read(&e)?).into_owned();
        parse_boot2(&text).map(|n| (n, text))
    }
}

/// Extrai o nome do ELF de `BOOT2 = cdrom0:\NOME;1`.
pub fn parse_boot2(text: &str) -> Result<String> {
    for line in text.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("BOOT2") {
            let v = rest.trim_start().strip_prefix('=').unwrap_or("").trim();
            let v = v.strip_prefix("cdrom0:").unwrap_or(v).trim_start_matches('\\');
            let name = v.split([';', ' ', '\t']).next().unwrap_or("");
            if !name.is_empty() {
                return Ok(name.to_string());
            }
        }
    }
    Err(err!("BOOT2 ausente em SYSTEM.CNF"))
}

fn read_user(f: &mut File, layout: Layout, lba: u64, buf: &mut [u8]) -> Result<()> {
    // `buf` e multiplo de 2048: leitura setor a setor so quando o setor nao e "cooked".
    if layout.sector == USER {
        f.seek(SeekFrom::Start(lba * USER))?;
        f.read_exact(buf)?;
        return Ok(());
    }
    let mut sector = vec![0u8; layout.sector as usize];
    for (i, chunk) in buf.chunks_mut(USER as usize).enumerate() {
        f.seek(SeekFrom::Start((lba + i as u64) * layout.sector))?;
        f.read_exact(&mut sector)?;
        chunk.copy_from_slice(&sector[layout.offset as usize..(layout.offset + USER) as usize]);
    }
    Ok(())
}

fn read_dir(f: &mut File, layout: Layout, lba: u32, size: u32) -> Result<Vec<Entry>> {
    let sectors = (size as u64).div_ceil(USER);
    let mut data = vec![0u8; (sectors * USER) as usize];
    read_user(f, layout, lba as u64, &mut data)?;
    let size = size as usize;
    let (mut o, mut out) = (0usize, Vec::new());
    while o < size {
        let rec_len = data[o] as usize;
        if rec_len == 0 {
            o = (o / USER as usize + 1) * USER as usize;
            continue;
        }
        if o + 34 > data.len() || o + rec_len > data.len() {
            return Err(err!("registro de diretorio corrompido (lba {lba})"));
        }
        let rec_lba = u32::from_le_bytes(data[o + 2..o + 6].try_into().unwrap());
        let rec_size = u32::from_le_bytes(data[o + 10..o + 14].try_into().unwrap());
        let is_dir = data[o + 25] & 2 != 0;
        let nlen = data[o + 32] as usize;
        let raw = &data[o + 33..o + 33 + nlen];
        if raw != [0] && raw != [1] {
            let name = String::from_utf8_lossy(raw);
            let name = name.split(';').next().unwrap_or("").to_string();
            out.push(Entry { name, lba: rec_lba, size: rec_size, is_dir });
        }
        o += rec_len;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot2_parsing() {
        assert_eq!(parse_boot2("BOOT2 = cdrom0:\\SLES_530.20;1\nVER = 1.00").unwrap(), "SLES_530.20");
        assert_eq!(parse_boot2("BOOT2=cdrom0:\\ABC_123.45;1").unwrap(), "ABC_123.45");
        assert!(parse_boot2("VER = 1.00").is_err());
    }
}
