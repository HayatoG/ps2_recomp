//! Inicia o port: monta o ambiente (`PS2X_*`), redireciona a saida para `logs/run_*.log` e espera.
use crate::{config::GameConfig, err, error::Result, info, log, warn};
use crate::options::Crt;
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
    /// Backend de desenho (`PS2X_GS`): `cpu`, `cpu-thread`, `d3d11` ou `hw`.
    pub renderer: Option<String>,
    /// Resolucao interna do renderer por hardware (`PS2X_GS_SCALE`, 1..8).
    pub internal_scale: Option<u32>,
    /// Filtro CRT (`PS2X_POSTFX_CRT` / `PS2X_POSTFX_CRT_STRENGTH`).
    pub crt: Option<Crt>,
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
    if let Some(n) = o.internal_scale {
        put("PS2X_GS_SCALE", n.to_string());
    }
    if o.vsync == Some(true) {
        put("PS2X_GS_VSYNC", "1".into());
    }
    if let Some(c) = o.crt {
        put("PS2X_POSTFX_CRT", if c == Crt::Off { "0" } else { "1" }.into());
        if let Some(st) = c.strength() {
            put("PS2X_POSTFX_CRT_STRENGTH", st.into());
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
    info!("port em execucao (pid {}); saida em {}", started.child.id(), started.run_log.display());
    Ok(None)
}

pub fn run(cfg: &GameConfig, o: &RunOptions) -> Result<i32> {
    let Started { mut child, run_log } = spawn(cfg, o)?;
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if let Some(t) = o.timeout_secs {
            if start.elapsed() > Duration::from_secs_f64(t) {
                let _ = child.kill();
                let _ = child.wait();
                warn!("tempo limite de {t}s atingido; processo encerrado. Saida em {}", run_log.display());
                return Ok(124);
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let code = status.code().unwrap_or(-1);
    info!("encerrado com codigo {code}; saida em {}", run_log.display());
    Ok(code)
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
            internal_scale: Some(4),
            crt: Some(Crt::Forte),
            vfs_cache_mb: Some(0),
            widescreen: true,
            ..Default::default()
        };
        let env = build_env_with(&cfg(), &o, &no_env);
        assert_eq!(env["PS2X_GS_SCALE"], "4");
        assert_eq!(env["PS2X_POSTFX_CRT"], "1");
        assert_eq!(env["PS2X_POSTFX_CRT_STRENGTH"], "1.0");
        assert_eq!(env["PS2X_VFS_CACHE_MB"], "0");
        assert_eq!(env["PS2X_WIDESCREEN"], "1");
        let env = build_env_with(&cfg(), &RunOptions { crt: Some(Crt::Off), ..Default::default() }, &no_env);
        assert_eq!(env["PS2X_POSTFX_CRT"], "0");
        assert!(!env.contains_key("PS2X_POSTFX_CRT_STRENGTH"));
        assert!(!env.contains_key("PS2X_VFS_CACHE_MB")); // ausente = automatico
    }

    #[test]
    fn args_expand() {
        let a = expand_args(&cfg(), &["{elf}".into()]);
        assert!(a[0].ends_with("SLES_000.00"));
    }
}
