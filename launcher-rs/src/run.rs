//! Inicia o port: monta o ambiente (`PS2X_*`), redireciona a saida para `logs/run_*.log` e espera.
use crate::{config::GameConfig, err, error::Result, info, log, warn};
use crate::options::{self, Bands, Crt};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Opcoes de execucao (cada uma vira uma variavel `PS2X_*` do runtime).
#[derive(Default, Debug, Clone)]
pub struct RunOptions {
    pub fps: Option<u32>,
    pub pad_script: Option<String>,
    pub fps_log: Option<String>,
    pub shot_dir: Option<String>,
    pub missing_log: Option<String>,
    pub heartbeat: bool,
    pub peek: Option<String>,
    pub timeout_secs: Option<f64>,
    /// Backend de desenho (`PS2X_GS`): `cpu`, `cpu-thread`, `hw` (avancado: `cpu-bands`, `d3d11`, `hw-serial`).
    pub renderer: Option<String>,
    /// Resolucao interna do renderer por hardware (`PS2X_GS_SCALE`, 1..8).
    pub internal_scale: Option<u32>,
    /// Filtro CRT (`PS2X_POSTFX_CRT` / `PS2X_POSTFX_CRT_STRENGTH`).
    pub crt: Option<Crt>,
    /// Forca do CRT em percentual (`PS2X_POSTFX_CRT_STRENGTH`, so com CRT ligado).
    pub crt_strength: Option<u32>,
    /// Faixas de rasterizacao (`PS2X_GS_BANDS`; so cpu-thread/cpu-bands).
    pub bands: Option<Bands>,
    /// Fila do GS em KiB (`PS2X_GS_QUEUE_KB`).
    pub queue_kb: Option<u32>,
    /// Cache do VFS em MiB (`PS2X_VFS_CACHE_MB`); 0 desliga; `None` = automatico.
    pub vfs_cache_mb: Option<u32>,
    /// Reservado (`PS2X_WIDESCREEN=1`): o runtime ainda nao tem efeito.
    pub widescreen: bool,
    /// Variaveis `PS2X_*` cujo valor veio do `launcher.cfg` (e nao da linha de comando): o
    /// ambiente do processo vence essas (precedencia: CLI > ambiente > cfg > game.toml).
    pub from_cfg: BTreeSet<&'static str>,
    /// Sincronia vertical do D3D11 (`PS2X_GS_VSYNC`).
    pub vsync: Option<bool>,
    /// Sem janela de console para o port (a GUI nao tem console; a saida ja vai para o log).
    pub no_console: bool,
    pub extra_args: Vec<String>,
}

/// Monta o ambiente do port. Precedencia: linha de comando > ambiente do processo >
/// `launcher.cfg` > `game.toml`. Variaveis ja presentes no ambiente nao sao sobrescritas pelo
/// cfg nem pelo `game.toml`; so a linha de comando as sobrescreve.
pub fn build_env(cfg: &GameConfig, o: &RunOptions) -> BTreeMap<String, String> {
    build_env_with(cfg, o, &|k| std::env::var(k).ok())
}

pub fn build_env_with(cfg: &GameConfig, o: &RunOptions, process_env: &dyn Fn(&str) -> Option<String>) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    // game.toml (menor precedencia): o ambiente do usuario vence.
    for (k, v) in std::iter::once(("PS2X_VIDEO_HZ".to_string(), cfg.video_hz().to_string())).chain(cfg.run_env.clone()) {
        if process_env(&k).is_none() {
            env.insert(k, v);
        }
    }
    let abs = |p: &str| std::path::absolute(p).map(|x| x.display().to_string()).unwrap_or_else(|_| p.to_string());
    // Valor vindo da CLI sempre entra; valor vindo do cfg so se o ambiente nao trouxer a variavel.
    let mut put = |k: &'static str, v: String| {
        if o.from_cfg.contains(k) && process_env(k).is_some() {
            info!("{k} ja definido no ambiente: vence o launcher.cfg");
        } else {
            env.insert(k.to_string(), v);
        }
    };
    if let Some(v) = o.fps {
        put("PS2X_TARGET_FPS", v.to_string()); // 0 = ilimitado
    }
    if let Some(v) = &o.pad_script {
        put("PS2X_PAD_SCRIPT", abs(v));
    }
    if let Some(v) = &o.fps_log {
        put("PS2X_FPS_LOG", abs(v));
    }
    if let Some(v) = &o.shot_dir {
        put("PS2X_SHOT_DIR", abs(v));
    }
    if let Some(v) = &o.missing_log {
        put("PS2X_MISSING_LOG", abs(v));
    }
    if o.heartbeat || o.peek.is_some() {
        put("PS2X_HEARTBEAT", "1".into());
    }
    if let Some(v) = &o.peek {
        put("PS2X_PEEK", v.clone());
    }
    if let Some(v) = &o.renderer {
        put("PS2X_GS", v.clone());
    }
    // Renderizador efetivo: o da CLI/cfg ou, na falta, o do ambiente (o runtime assume cpu).
    let env_gs = process_env("PS2X_GS");
    let from_cli = o.renderer.is_some() && !o.from_cfg.contains("PS2X_GS");
    let effective = if from_cli { o.renderer.clone() } else { env_gs.or_else(|| o.renderer.clone()) };
    let effective = effective.unwrap_or_else(|| "cpu".into()).to_ascii_lowercase();
    if let Some(n) = o.internal_scale {
        if options::is_hw(&effective) {
            put("PS2X_GS_SCALE", n.to_string());
        } else if n > 1 {
            warn!("escala interna {n} ignorada: so vale com hw (renderizador atual: {effective})");
        }
    }
    if let Some(b) = o.bands {
        if options::uses_bands(&effective) {
            put("PS2X_GS_BANDS", b.env());
        } else {
            warn!("faixas ({}) ignoradas: so valem com cpu-thread/cpu-bands (renderizador atual: {effective})", b.key());
        }
    }
    if let Some(n) = o.queue_kb {
        put("PS2X_GS_QUEUE_KB", n.to_string());
    }
    if o.vsync == Some(true) {
        put("PS2X_GS_VSYNC", "1".into());
    }
    if let Some(c) = o.crt {
        // O runtime entende os niveis pelo nome (suave|medio|forte); a forca so quando pedida.
        put("PS2X_POSTFX_CRT", if c == Crt::Off { "0".into() } else { c.key().to_string() });
        if c != Crt::Off {
            if let Some(p) = o.crt_strength {
                put("PS2X_POSTFX_CRT_STRENGTH", options::crt_strength_env(p));
            }
        }
    }
    if let Some(n) = o.vfs_cache_mb {
        put("PS2X_VFS_CACHE_MB", n.to_string());
    }
    if o.widescreen {
        put("PS2X_WIDESCREEN", "1".into()); // planejado: ainda sem efeito no runtime
    }
    env
}

/// Linha do log do runtime que indica que o renderer hw nao iniciou e caiu para a CPU
/// (`[gs-hw] ... using cpu`).
pub fn hw_fallback_line(log: &str) -> Option<&str> {
    log.lines().find(|l| l.contains("[gs-hw]") && l.contains("using cpu"))
}

/// Avisa (log do launcher) se o log do port mostra que o hw caiu para a CPU.
pub fn warn_if_hw_fell_back(run_log: &std::path::Path) {
    let text = std::fs::read_to_string(run_log).unwrap_or_default();
    if let Some(l) = hw_fallback_line(&text) {
        warn!("o renderer hw nao iniciou e o jogo usa a CPU: {} -- use --renderer cpu-thread", l.trim());
    }
}

/// Substitui `{elf}` e `{work}` nos argumentos do `game.toml`.
pub fn expand_args(cfg: &GameConfig, args: &[String]) -> Vec<String> {
    let elf = cfg.work_dir.join(&cfg.serial).display().to_string();
    let work = cfg.work_dir.display().to_string();
    args.iter().map(|a| a.replace("{elf}", &elf).replace("{work}", &work)).collect()
}

/// Caminho do executavel do port, se existir.
pub fn executable(cfg: &GameConfig) -> Result<&std::path::Path> {
    let exe = cfg.run_executable.as_ref().ok_or_else(|| err!("game.toml nao define [run].executable (port ainda nao construido)"))?;
    if !exe.is_file() {
        return Err(err!("executavel do port nao encontrado: {}", exe.display()));
    }
    Ok(exe)
}

/// Processo do port iniciado, com o log da sua saida.
pub struct Started {
    pub child: Child,
    pub run_log: PathBuf,
}

/// Inicia o port sem esperar (a GUI fecha logo depois; o processo segue sozinho).
pub fn spawn(cfg: &GameConfig, o: &RunOptions) -> Result<Started> {
    let exe = executable(cfg)?;
    let mut args = expand_args(cfg, &cfg.run_args);
    args.extend(o.extra_args.iter().cloned());
    let env = build_env(cfg, o);
    let logs = cfg.root.join("logs");
    std::fs::create_dir_all(&logs)?;
    let run_log = logs.join(format!("run_{}.log", log::stamp()));
    info!("iniciando: {} {:?}", exe.display(), args);
    info!("opcoes de execucao: {:?}", env);
    let out = std::fs::File::create(&run_log)?;
    let mut cmd = Command::new(exe);
    cmd.args(&args)
        .current_dir(&cfg.root)
        .envs(&env)
        .stdout(Stdio::from(out.try_clone()?))
        .stderr(Stdio::from(out));
    #[cfg(windows)]
    if o.no_console {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd
        .spawn()
        .map_err(|e| err!("nao consegui iniciar {}: {e}", exe.display()))?;
    Ok(Started { child, run_log })
}

/// Espera ate `grace` para ver se o port caiu logo ao abrir. `Ok(None)` = segue rodando (ou saiu
/// com codigo 0); `Err` = saiu com erro, com o fim do log na mensagem.
pub fn check_started(started: &mut Started, grace: Duration) -> Result<Option<i32>> {
    let t0 = Instant::now();
    while t0.elapsed() < grace {
        if let Some(s) = started.child.try_wait()? {
            let code = s.code().unwrap_or(-1);
            if code == 0 {
                return Ok(Some(0));
            }
            let text = std::fs::read_to_string(&started.run_log).unwrap_or_default();
            let tail: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).rev().take(4).collect();
            let tail: Vec<&str> = tail.into_iter().rev().collect();
            let name = started.run_log.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let mut msg = format!("o jogo fechou ao iniciar (codigo {code}); saida em logs\\{name}");
            if !tail.is_empty() {
                msg.push('\n');
                msg.push_str(&tail.join("\n"));
            }
            return Err(err!("{msg}"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    warn_if_hw_fell_back(&started.run_log);
    info!("port em execucao (pid {}); saida em {}", started.child.id(), started.run_log.display());
    Ok(None)
}

/// Janela em que uma saida com erro conta como "o hw nao iniciou" (fallback para cpu-thread).
const EARLY_EXIT: Duration = Duration::from_secs(20);

/// Roda o port e espera. Se o renderer hw (escolhido por cfg/perfil ou CLI) fechar com erro logo
/// ao abrir, tenta uma vez de novo com `cpu-thread` e avisa no log.
pub fn run(cfg: &GameConfig, o: &RunOptions) -> Result<i32> {
    let (code, elapsed) = run_once(cfg, o)?;
    let was_hw = o.renderer.as_deref().is_some_and(options::is_hw);
    if code != 0 && code != 124 && was_hw && elapsed < EARLY_EXIT {
        warn!("o renderer hw fechou com erro (codigo {code}) em {:.1}s; tentando de novo com cpu-thread", elapsed.as_secs_f64());
        let mut fb = o.clone();
        fb.renderer = Some("cpu-thread".into());
        fb.internal_scale = None;
        return run_once(cfg, &fb).map(|r| r.0);
    }
    Ok(code)
}

fn run_once(cfg: &GameConfig, o: &RunOptions) -> Result<(i32, Duration)> {
    let Started { mut child, run_log } = spawn(cfg, o)?;
    let start = Instant::now();
    let mut scanned = false;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if !scanned && start.elapsed() > Duration::from_secs(8) {
            scanned = true;
            warn_if_hw_fell_back(&run_log);
        }
        if let Some(t) = o.timeout_secs {
            if start.elapsed() > Duration::from_secs_f64(t) {
                let _ = child.kill();
                let _ = child.wait();
                warn_if_hw_fell_back(&run_log);
                warn!("tempo limite de {t}s atingido; processo encerrado. Saida em {}", run_log.display());
                return Ok((124, start.elapsed()));
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let code = status.code().unwrap_or(-1);
    warn_if_hw_fell_back(&run_log);
    info!("encerrado com codigo {code}; saida em {}", run_log.display());
    Ok((code, start.elapsed()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> GameConfig {
        GameConfig {
            root: ".".into(),
            name: "X".into(),
            serial: "SLES_000.00".into(),
            region: "PAL".into(),
            iso_glob: "*.iso".into(),
            boot_elf_sha256: String::new(),
            dump_files: vec![],
            work_dir: "w".into(),
            extract_files: vec![],
            run_executable: None,
            run_args: vec!["{elf}".into()],
            run_env: BTreeMap::from([("PS2X_TARGET_FPS".to_string(), "60".to_string())]),
            art_focus: [0.5, 0.5],
        }
    }

    #[test]
    fn env_defaults_and_overrides() {
        let o = RunOptions { fps: Some(30), heartbeat: true, ..Default::default() };
        let env = build_env_with(&cfg(), &o, &|_| None);
        assert_eq!(env["PS2X_VIDEO_HZ"], "50");
        assert_eq!(env["PS2X_TARGET_FPS"], "30"); // a linha de comando vence o game.toml
        assert_eq!(env["PS2X_HEARTBEAT"], "1");
        assert!(!env.contains_key("PS2X_GS"));
        let o = RunOptions { renderer: Some("d3d11".into()), vsync: Some(true), ..Default::default() };
        let env = build_env_with(&cfg(), &o, &|_| None);
        assert_eq!(env["PS2X_GS"], "d3d11");
        assert_eq!(env["PS2X_GS_VSYNC"], "1");
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn cli_beats_env_beats_cfg_beats_game_toml() {
        let user = |k: &str| match k {
            "PS2X_GS" => Some("hw".to_string()),
            "PS2X_TARGET_FPS" => Some("25".to_string()),
            _ => None,
        };
        // Valores do cfg: o ambiente vence; game.toml (PS2X_TARGET_FPS=60) tambem perde para o ambiente.
        let o = RunOptions {
            renderer: Some("cpu".into()),
            internal_scale: Some(3),
            from_cfg: BTreeSet::from(["PS2X_GS", "PS2X_GS_SCALE"]),
            ..Default::default()
        };
        let env = build_env_with(&cfg(), &o, &user);
        assert!(!env.contains_key("PS2X_GS"), "ambiente deve vencer o cfg");
        assert!(!env.contains_key("PS2X_TARGET_FPS"), "ambiente deve vencer o game.toml");
        assert_eq!(env["PS2X_GS_SCALE"], "3"); // sem ambiente: o cfg vale
        // Linha de comando vence o ambiente.
        let o = RunOptions { renderer: Some("d3d11".into()), fps: Some(30), ..Default::default() };
        let env = build_env_with(&cfg(), &o, &user);
        assert_eq!((env["PS2X_GS"].as_str(), env["PS2X_TARGET_FPS"].as_str()), ("d3d11", "30"));
        // Sem ambiente, o cfg vence o game.toml.
        let o = RunOptions { fps: Some(45), from_cfg: BTreeSet::from(["PS2X_TARGET_FPS"]), ..Default::default() };
        assert_eq!(build_env_with(&cfg(), &o, &no_env)["PS2X_TARGET_FPS"], "45");
    }

    #[test]
    fn new_options_export() {
        let o = RunOptions {
            renderer: Some("hw".into()),
            internal_scale: Some(4),
            crt: Some(Crt::Forte),
            vfs_cache_mb: Some(0),
            widescreen: true,
            ..Default::default()
        };
        let env = build_env_with(&cfg(), &o, &no_env);
        assert_eq!(env["PS2X_GS_SCALE"], "4");
        assert_eq!(env["PS2X_POSTFX_CRT"], "forte");
        assert!(!env.contains_key("PS2X_POSTFX_CRT_STRENGTH")); // so quando pedida
        assert_eq!(env["PS2X_VFS_CACHE_MB"], "0");
        assert_eq!(env["PS2X_WIDESCREEN"], "1");
        let env = build_env_with(&cfg(), &RunOptions { crt: Some(Crt::Off), ..Default::default() }, &no_env);
        assert_eq!(env["PS2X_POSTFX_CRT"], "0");
        assert!(!env.contains_key("PS2X_POSTFX_CRT_STRENGTH"));
        assert!(!env.contains_key("PS2X_VFS_CACHE_MB")); // ausente = automatico
    }

    #[test]
    fn scale_bands_depend_on_renderer() {
        let mk = |r: &str| RunOptions {
            renderer: Some(r.into()),
            internal_scale: Some(2),
            bands: Some(Bands::Auto),
            queue_kb: Some(8192),
            crt: Some(Crt::Medio),
            crt_strength: Some(50),
            ..Default::default()
        };
        let hw = build_env_with(&cfg(), &mk("hw"), &no_env);
        assert_eq!(hw["PS2X_GS_SCALE"], "2");
        assert!(!hw.contains_key("PS2X_GS_BANDS"), "nunca bands com hw");
        assert_eq!((hw["PS2X_GS_QUEUE_KB"].as_str(), hw["PS2X_POSTFX_CRT"].as_str(), hw["PS2X_POSTFX_CRT_STRENGTH"].as_str()), ("8192", "medio", "0.50"));
        let ct = build_env_with(&cfg(), &mk("cpu-thread"), &no_env);
        assert_eq!(ct["PS2X_GS_BANDS"], "auto");
        assert!(!ct.contains_key("PS2X_GS_SCALE"));
        // Renderizador vindo so do ambiente tambem conta.
        let o = RunOptions { internal_scale: Some(3), bands: Some(Bands::N(4)), ..Default::default() };
        let e = build_env_with(&cfg(), &o, &|k| (k == "PS2X_GS").then(|| "hw-serial".to_string()));
        assert_eq!(e["PS2X_GS_SCALE"], "3");
        assert!(!e.contains_key("PS2X_GS_BANDS"));
        // CRT desligado nao exporta forca.
        let o = RunOptions { crt: Some(Crt::Off), crt_strength: Some(10), ..Default::default() };
        assert!(!build_env_with(&cfg(), &o, &no_env).contains_key("PS2X_POSTFX_CRT_STRENGTH"));
    }

    #[test]
    fn detects_hw_fallback_line() {
        assert!(hw_fallback_line("a
[gs-hw] init failed, using cpu
b").is_some());
        assert!(hw_fallback_line("[gs-hw] ok using gpu").is_none());
        assert!(hw_fallback_line("using cpu").is_none());
    }

    #[test]
    fn args_expand() {
        let a = expand_args(&cfg(), &["{elf}".into()]);
        assert!(a[0].ends_with("SLES_000.00"));
    }
}
