//! Inicia o port: monta o ambiente (`PS2X_*`), redireciona a saida para `logs/run_*.log` e espera.
use crate::{config::GameConfig, err, error::Result, info, log, warn};
use std::collections::BTreeMap;
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
    /// Backend de desenho (`PS2X_GS`): `cpu` ou `d3d11`.
    pub renderer: Option<String>,
    /// Sincronia vertical do D3D11 (`PS2X_GS_VSYNC`).
    pub vsync: Option<bool>,
    /// Sem janela de console para o port (a GUI nao tem console; a saida ja vai para o log).
    pub no_console: bool,
    pub extra_args: Vec<String>,
}

pub fn build_env(cfg: &GameConfig, o: &RunOptions) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("PS2X_VIDEO_HZ".into(), cfg.video_hz().into());
    env.extend(cfg.run_env.clone());
    let abs = |p: &str| std::path::absolute(p).map(|x| x.display().to_string()).unwrap_or_else(|_| p.to_string());
    if let Some(v) = o.fps {
        env.insert("PS2X_TARGET_FPS".into(), v.to_string()); // 0 = ilimitado
    }
    if let Some(v) = &o.pad_script {
        env.insert("PS2X_PAD_SCRIPT".into(), abs(v));
    }
    if let Some(v) = &o.fps_log {
        env.insert("PS2X_FPS_LOG".into(), abs(v));
    }
    if let Some(v) = &o.shot_dir {
        env.insert("PS2X_SHOT_DIR".into(), abs(v));
    }
    if let Some(v) = &o.missing_log {
        env.insert("PS2X_MISSING_LOG".into(), abs(v));
    }
    if o.heartbeat || o.peek.is_some() {
        env.insert("PS2X_HEARTBEAT".into(), "1".into());
    }
    if let Some(v) = &o.peek {
        env.insert("PS2X_PEEK".into(), v.clone());
    }
    if let Some(v) = &o.renderer {
        env.insert("PS2X_GS".into(), v.clone());
    }
    if o.vsync == Some(true) {
        env.insert("PS2X_GS_VSYNC".into(), "1".into());
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
        let env = build_env(&cfg(), &o);
        assert_eq!(env["PS2X_VIDEO_HZ"], "50");
        assert_eq!(env["PS2X_TARGET_FPS"], "30"); // a linha de comando vence o game.toml
        assert_eq!(env["PS2X_HEARTBEAT"], "1");
        assert!(!env.contains_key("PS2X_GS"));
        let o = RunOptions { renderer: Some("d3d11".into()), vsync: Some(true), ..Default::default() };
        let env = build_env(&cfg(), &o);
        assert_eq!(env["PS2X_GS"], "d3d11");
        assert_eq!(env["PS2X_GS_VSYNC"], "1");
    }

    #[test]
    fn args_expand() {
        let a = expand_args(&cfg(), &["{elf}".into()]);
        assert!(a[0].ends_with("SLES_000.00"));
    }
}
