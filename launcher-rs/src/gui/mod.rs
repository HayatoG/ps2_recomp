//! `ps2launch-gui`: a janela do launcher (egui). Usa o mesmo nucleo do CLI.
//!
//! ```text
//! ps2launch-gui [--game DIR] [--settings | --no-window | --check]
//! ```
//! - sem flags: abre a janela (ou joga direto se `show_window = 0` no launcher.cfg);
//! - `--settings`: sempre abre a janela;
//! - `--no-window`: valida, extrai o que falta e inicia o port com as configuracoes salvas;
//! - `--check`: valida o dump e o port e sai com 0 (pronto) ou 2 (nao pronto).
mod app;
mod art;
mod pad;
pub mod strings;
mod theme;
mod widgets;
mod worker;

use crate::settings::{self, Settings};
use crate::{assets, config::GameConfig, dump, err, error::Result, info, log, run, warn};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

#[derive(Debug, PartialEq)]
enum Mode {
    Default,
    Settings,
    NoWindow,
    Check,
}

struct Args {
    game: Option<PathBuf>,
    mode: Mode,
}

fn parse(argv: Vec<String>) -> Result<Args> {
    let mut a = Args { game: None, mode: Mode::Default };
    let mut it = argv.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--game" => a.game = Some(it.next().ok_or_else(|| err!("--game precisa de um valor"))?.into()),
            "--settings" => a.mode = Mode::Settings,
            "--no-window" => a.mode = Mode::NoWindow,
            "--check" => a.mode = Mode::Check,
            s => return Err(err!("argumento desconhecido: {s} (use --game DIR, --settings, --no-window, --check)")),
        }
    }
    Ok(a)
}

/// Pasta do jogo: `--game`, senao a atual, senao a do executavel (atalho da Steam).
fn game_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(std::path::absolute(p)?);
    }
    let cwd = std::env::current_dir()?;
    if cwd.join("game.toml").is_file() {
        return Ok(cwd);
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        if dir.join("game.toml").is_file() {
            return Ok(dir);
        }
    }
    Ok(cwd)
}

/// Sem console proprio (subsistema windows): usa o do terminal que chamou, se houver, para
/// `--check` imprimir o resultado.
fn attach_parent_console() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn AttachConsole(pid: u32) -> i32;
        }
        const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
        // SAFETY: chamada simples da API do Windows, sem ponteiros.
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

fn fatal_dialog(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("ps2launch")
        .set_description(msg)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Valida dump e port (e informa os dados extraidos); devolve o caminho da imagem.
fn check(cfg: &GameConfig, s: &Settings) -> Result<PathBuf> {
    let iso = dump::find_iso(cfg, s.iso.as_deref())?;
    dump::verify(cfg, &iso)?;
    dump::verify_sizes(cfg, &iso)?;
    let exe = run::executable(cfg)?;
    info!("port: {}", exe.display());
    let st = assets::local_status(cfg);
    info!("dados extraidos: {}/{} arquivos", st.present, st.total);
    Ok(iso)
}

fn play_without_window(cfg: &GameConfig, s: &Settings) -> Result<()> {
    let iso = check(cfg, s)?;
    assets::extract(cfg, &iso, false)?;
    let mut opts = run::RunOptions { no_console: true, ..Default::default() };
    s.apply(&mut opts);
    let mut started = run::spawn(cfg, &opts)?;
    run::check_started(&mut started, Duration::from_millis(1500))?;
    Ok(())
}

fn window(cfg: GameConfig, s: Settings) -> Result<()> {
    let title = format!("{} — {}", cfg.name, strings::get().app_suffix);
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(&title)
        .with_inner_size([880.0, 640.0])
        .with_min_inner_size([880.0, 640.0])
        .with_max_inner_size([880.0, 640.0])
        .with_resizable(false)
        .with_maximize_button(false);
    if let Some(icon) = art::window_icon(&cfg.root) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    eframe::run_native(&title, options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, cfg, s)))))
        .map_err(|e| err!("nao consegui abrir a janela: {e}"))
}

pub fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1).collect()) {
        Ok(a) => a,
        Err(e) => {
            attach_parent_console();
            eprintln!("erro: {e}");
            return ExitCode::from(2);
        }
    };
    if args.mode == Mode::Check {
        attach_parent_console();
    }
    let game = match game_dir(args.game) {
        Ok(g) => g,
        Err(e) => {
            fatal_dialog(&e.0);
            return ExitCode::from(2);
        }
    };
    log::init(&game.join("logs"));
    let cfg = match GameConfig::load(&game) {
        Ok(c) => c,
        Err(e) => {
            crate::error!("{e}");
            if args.mode != Mode::Check {
                fatal_dialog(&e.0);
            }
            return ExitCode::from(2);
        }
    };
    info!("ps2launch-gui: {} ({}) modo {:?}", cfg.name, cfg.serial, args.mode);
    let s = match Settings::load_or_create(&game) {
        Ok((s, warnings)) => {
            for w in warnings {
                warn!("{}: {w}", settings::FILE_NAME);
            }
            s
        }
        Err(e) => {
            warn!("{e}; usando o padrao");
            Settings::default()
        }
    };
    let mode = match args.mode {
        Mode::Default if s.show_window == Some(false) => Mode::NoWindow,
        m => m,
    };
    let result = match mode {
        Mode::Check => check(&cfg, &s).map(|_| info!("pronto para jogar")),
        Mode::NoWindow => play_without_window(&cfg, &s),
        Mode::Default | Mode::Settings => window(cfg, s),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            crate::error!("{e}");
            if mode == Mode::NoWindow {
                fatal_dialog(&e.0);
            }
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_parse() {
        let a = parse(vec!["--game".into(), "x".into(), "--check".into()]).unwrap();
        assert_eq!(a.game, Some(PathBuf::from("x")));
        assert_eq!(a.mode, Mode::Check);
        assert!(parse(vec!["--nope".into()]).is_err());
        assert!(parse(vec!["--game".into()]).is_err());
    }
}
