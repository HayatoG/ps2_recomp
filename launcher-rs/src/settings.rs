//! `launcher.cfg`: preferencias do jogador ao lado do `game.toml` (`chave = valor`, `#` comenta).
//!
//! - Gerado na primeira execucao com todas as chaves comentadas e explicadas ([`template`]).
//! - Chave ausente = "nao definida": vale o padrao do `game.toml`/runtime.
//! - Precedencia: flag de linha de comando > `launcher.cfg` > padrao ([`Settings::apply`]).
//! - Chave desconhecida ou valor invalido vira aviso (nunca derruba o launcher).
use crate::{error::Result, run::RunOptions};
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "launcher.cfg";

/// Perfil de imagem escolhido nos cartoes da aba Jogar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Original,
    Recommended,
    Deck,
    Custom,
}

impl Profile {
    pub const ALL: [Profile; 4] = [Profile::Original, Profile::Recommended, Profile::Deck, Profile::Custom];

    pub fn key(self) -> &'static str {
        match self {
            Profile::Original => "original",
            Profile::Recommended => "recommended",
            Profile::Deck => "deck",
            Profile::Custom => "custom",
        }
    }

    fn parse(v: &str) -> Option<Profile> {
        Profile::ALL.into_iter().find(|p| p.key() == v)
    }
}

/// Backend de desenho do runtime (`PS2X_GS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    /// Rasterizacao por software (padrao do runtime).
    Cpu,
    /// Desenha na CPU e apresenta via Direct3D 11 (experimental).
    D3d11,
}

impl Renderer {
    pub fn key(self) -> &'static str {
        match self {
            Renderer::Cpu => "cpu",
            Renderer::D3d11 => "d3d11",
        }
    }

    pub fn parse(v: &str) -> Option<Renderer> {
        match v {
            "cpu" => Some(Renderer::Cpu),
            "d3d11" => Some(Renderer::D3d11),
            _ => None,
        }
    }
}

/// Limites de FPS oferecidos (0 = ilimitado) e os do perfil Steam Deck.
pub const FPS_CHOICES: [u32; 4] = [30, 50, 60, 0];
pub const DECK_FPS_CHOICES: [u32; 3] = [30, 40, 60];

/// Valores lidos do `launcher.cfg`; `None` = chave ausente.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    pub iso: Option<PathBuf>,
    pub profile: Option<Profile>,
    pub renderer: Option<Renderer>,
    pub fps: Option<u32>,
    pub deck_fps: Option<u32>,
    pub vsync: Option<bool>,
    pub heartbeat: Option<bool>,
    /// `false` faz o launcher sem argumentos jogar direto (como `--no-window`).
    pub show_window: Option<bool>,
}

/// O que vai para o runtime depois de aplicar o perfil (so o que foi definido).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Resolved {
    pub renderer: Option<Renderer>,
    pub fps: Option<u32>,
    pub vsync: Option<bool>,
    pub heartbeat: bool,
}

fn parse_bool(v: &str) -> Option<bool> {
    match v {
        "1" | "true" | "sim" | "yes" | "on" => Some(true),
        "0" | "false" | "nao" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn bool_text(b: bool) -> &'static str {
    if b {
        "1"
    } else {
        "0"
    }
}

impl Settings {
    /// Interpreta o texto; devolve as configuracoes e os avisos (linha + motivo).
    pub fn parse(text: &str) -> (Settings, Vec<String>) {
        let mut s = Settings::default();
        let mut warnings = Vec::new();
        for (n, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                warnings.push(format!("linha {}: esperado 'chave = valor'", n + 1));
                continue;
            };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let lower = v.to_ascii_lowercase();
            let ok = match k.as_str() {
                "iso" => {
                    s.iso = (!v.is_empty()).then(|| PathBuf::from(v.trim_matches('"')));
                    true
                }
                "profile" => Profile::parse(&lower).map(|p| s.profile = Some(p)).is_some(),
                "renderer" => Renderer::parse(&lower).map(|r| s.renderer = Some(r)).is_some(),
                "fps" => lower.parse().ok().map(|f| s.fps = Some(f)).is_some(),
                "deck_fps" => lower.parse().ok().filter(|f| DECK_FPS_CHOICES.contains(f)).map(|f| s.deck_fps = Some(f)).is_some(),
                "vsync" => parse_bool(&lower).map(|b| s.vsync = Some(b)).is_some(),
                "heartbeat" => parse_bool(&lower).map(|b| s.heartbeat = Some(b)).is_some(),
                "show_window" => parse_bool(&lower).map(|b| s.show_window = Some(b)).is_some(),
                _ => {
                    warnings.push(format!("linha {}: chave desconhecida '{k}'", n + 1));
                    continue;
                }
            };
            if !ok {
                warnings.push(format!("linha {}: valor invalido para {k}: '{v}'", n + 1));
            }
        }
        (s, warnings)
    }

    /// Aplica o perfil: Original/Recomendado/Deck fixam renderer, fps e vsync; Personalizado (ou
    /// sem perfil) usa as chaves soltas.
    pub fn resolve(&self) -> Resolved {
        let heartbeat = self.heartbeat.unwrap_or(false);
        match self.profile {
            Some(Profile::Original) | Some(Profile::Recommended) => {
                Resolved { renderer: Some(Renderer::Cpu), fps: Some(60), vsync: Some(false), heartbeat }
            }
            Some(Profile::Deck) => Resolved {
                renderer: Some(Renderer::Cpu),
                fps: Some(self.deck_fps.unwrap_or(60)),
                vsync: Some(false),
                heartbeat,
            },
            Some(Profile::Custom) | None => {
                Resolved { renderer: self.renderer, fps: self.fps, vsync: self.vsync, heartbeat }
            }
        }
    }

    /// Preenche em `o` so o que a linha de comando nao definiu (flag > cfg > padrao).
    pub fn apply(&self, o: &mut RunOptions) {
        let r = self.resolve();
        o.fps = o.fps.or(r.fps);
        o.renderer = o.renderer.clone().or(r.renderer.map(|x| x.key().to_string()));
        o.vsync = o.vsync.or(r.vsync);
        o.heartbeat |= r.heartbeat;
    }

    /// Texto completo, com as chaves definidas ativas e as demais comentadas.
    pub fn render(&self) -> String {
        let mut out = String::from(HEADER);
        let iso = self.iso.as_ref().map(|p| p.display().to_string());
        let entries: [(&str, Option<String>, &str); 8] = [
            ("iso", iso, "Caminho da imagem do disco (ISO/BIN). Vazio = procurar pelo iso_glob na pasta do jogo."),
            ("profile", self.profile.map(|p| p.key().into()), "Perfil: original | recommended | deck | custom (custom usa renderer/fps/vsync abaixo)."),
            ("renderer", self.renderer.map(|r| r.key().into()), "Desenho: cpu (padrao, software) | d3d11 (experimental: so apresenta; desenho ainda na CPU)."),
            ("fps", self.fps.map(|f| f.to_string()), "Limite de FPS: 30 | 50 | 60 | 0 (0 = ilimitado)."),
            ("deck_fps", self.deck_fps.map(|f| f.to_string()), "Limite do perfil Steam Deck: 30 | 40 | 60."),
            ("vsync", self.vsync.map(|b| bool_text(b).into()), "Sincronia vertical do Direct3D 11 (PS2X_GS_VSYNC): 0 | 1."),
            ("heartbeat", self.heartbeat.map(|b| bool_text(b).into()), "Diagnostico: uma linha por segundo no log do jogo (PS2X_HEARTBEAT): 0 | 1."),
            ("show_window", self.show_window.map(|b| bool_text(b).into()), "1 = abrir esta janela ao iniciar; 0 = jogar direto (use --settings para reabrir)."),
        ];
        for (key, value, help) in entries {
            out.push_str(&format!("\n# {help}\n"));
            match value {
                Some(v) => out.push_str(&format!("{key} = {v}\n")),
                None => out.push_str(&format!("# {key} = {}\n", default_text(key))),
            }
        }
        out
    }

    /// Le `<jogo>/launcher.cfg`; se nao existir, grava o modelo comentado e devolve o padrao.
    pub fn load_or_create(game_dir: &Path) -> Result<(Settings, Vec<String>)> {
        let path = game_dir.join(FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Settings::parse(&text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                write_atomic(&path, &template())?;
                Ok((Settings::default(), Vec::new()))
            }
            Err(e) => Err(crate::err!("nao consegui ler {}: {e}", path.display())),
        }
    }

    pub fn save(&self, game_dir: &Path) -> Result<()> {
        write_atomic(&game_dir.join(FILE_NAME), &self.render())
    }
}

const HEADER: &str = "# launcher.cfg - preferencias do ps2launch (chave = valor; '#' comenta).\n\
# Gerado pelo launcher; pode editar a mao. Flags de linha de comando vencem estes valores.\n\
# Chave comentada = usa o padrao.\n";

fn default_text(key: &str) -> &'static str {
    match key {
        "profile" => "recommended",
        "renderer" => "cpu",
        "fps" => "60",
        "deck_fps" => "60",
        "vsync" | "heartbeat" => "0",
        "show_window" => "1",
        _ => "",
    }
}

/// Modelo da primeira execucao: todas as chaves comentadas.
pub fn template() -> String {
    Settings::default().render()
}

fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_extension("cfg.part");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_is_all_comments_and_parses_to_default() {
        let t = template();
        assert!(t.lines().all(|l| l.is_empty() || l.starts_with('#')));
        let (s, w) = Settings::parse(&t);
        assert_eq!(s, Settings::default());
        assert!(w.is_empty());
    }

    #[test]
    fn parse_values_bom_and_warnings() {
        let (s, w) = Settings::parse(
            "\u{feff}iso = \"D:\\Jogos\\x.iso\"\nprofile = DECK\ndeck_fps=40\nvsync = 1\nfps = abc\nfoo = 1\nsem igual\n",
        );
        assert_eq!(s.iso, Some(PathBuf::from("D:\\Jogos\\x.iso")));
        assert_eq!(s.profile, Some(Profile::Deck));
        assert_eq!(s.deck_fps, Some(40));
        assert_eq!(s.vsync, Some(true));
        assert_eq!(s.fps, None);
        assert_eq!(w.len(), 3, "{w:?}");
    }

    #[test]
    fn render_round_trips() {
        let s = Settings {
            iso: Some("C:\\a b\\jogo.iso".into()),
            profile: Some(Profile::Custom),
            renderer: Some(Renderer::D3d11),
            fps: Some(0),
            deck_fps: Some(30),
            vsync: Some(true),
            heartbeat: Some(false),
            show_window: Some(true),
        };
        let (back, w) = Settings::parse(&s.render());
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(back, s);
    }

    #[test]
    fn profile_fixes_values_and_custom_uses_keys() {
        let mut s = Settings { profile: Some(Profile::Deck), deck_fps: Some(40), fps: Some(30), ..Default::default() };
        assert_eq!(s.resolve().fps, Some(40));
        s.profile = Some(Profile::Custom);
        s.renderer = Some(Renderer::D3d11);
        let r = s.resolve();
        assert_eq!((r.fps, r.renderer), (Some(30), Some(Renderer::D3d11)));
        s.profile = Some(Profile::Recommended);
        assert_eq!(s.resolve().renderer, Some(Renderer::Cpu));
    }

    #[test]
    fn precedence_flag_over_cfg_over_default() {
        let s = Settings { profile: Some(Profile::Custom), fps: Some(30), renderer: Some(Renderer::D3d11), heartbeat: Some(true), ..Default::default() };
        // Flag vence o cfg.
        let mut o = RunOptions { fps: Some(50), ..Default::default() };
        s.apply(&mut o);
        assert_eq!(o.fps, Some(50));
        assert_eq!(o.renderer.as_deref(), Some("d3d11"));
        assert!(o.heartbeat);
        // Sem flag, vale o cfg.
        let mut o = RunOptions::default();
        s.apply(&mut o);
        assert_eq!(o.fps, Some(30));
        // Sem cfg nem flag, nada e definido (vale o padrao do game.toml/runtime).
        let mut o = RunOptions::default();
        Settings::default().apply(&mut o);
        assert_eq!((o.fps, o.renderer, o.vsync), (None, None, None));
    }
}
