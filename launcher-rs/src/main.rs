//! CLI do launcher: `ps2launch <info|verify|extract|run> [--game DIR] [--iso ARQ] [opcoes]`.
use ps2launch::options;
use ps2launch::settings::{self, Settings};
use ps2launch::{assets, config::GameConfig, dump, err, error, error::Result, hw, info, log, run, warn};
use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "\
ps2launch - launcher leve para ports PS2 (regras 4-6: exige o dump original, valida, loga)

USO:
    ps2launch <comando> [opcoes] [-- args do runtime]

COMANDOS:
    info      mostra a configuracao lida do game.toml
    verify    valida o dump (serial + sha256 do ELF); com --deep, rehash dos arquivos extraidos
    hashes    imprime o bloco [[dump.file]] (tamanho + sha256) do manifesto, para o game.toml
    extract   extrai os dados do disco para <jogo>/work (atomico, com manifesto)
    run       valida, extrai o que falta e inicia o port

OPCOES:
    --game DIR        diretorio do jogo, com game.toml (padrao: .)
    --iso ARQ         caminho do dump (padrao: procura iso_glob em DIR)
    --force           refaz a extracao mesmo que os arquivos existam
    --deep            (verify) confere o hash de todos os arquivos extraidos
  Opcoes do comando run:
    --fps N           limite de FPS (0 = ilimitado)
    --pad-script ARQ  script do gamepad virtual
    --fps-log CSV     grava o FPS por segundo
    --shot-dir DIR    pasta dos PNG do comando 'shot' do pad virtual
    --missing-log ARQ registra alvos de desvio sem funcao gerada
    --renderer R      desenho (PS2X_GS): cpu (padrao) | cpu-thread | d3d11 | hw (em desenvolvimento)
    --internal-scale N  resolucao interna do renderer hw, 1..8 (PS2X_GS_SCALE; padrao 1)
    --crt [NIVEL]     filtro CRT: off | suave | medio | forte (sem NIVEL = medio)
    --vfs-cache-mb N  cache do VFS em MiB (PS2X_VFS_CACHE_MB; 0 desliga; ausente = automatico)
    --widescreen      reservado (PS2X_WIDESCREEN=1): planejado, ainda sem efeito
    --vsync           sincronia vertical do d3d11
    --heartbeat       uma linha por segundo com o estado do guest (diagnostico)
    --peek 0xA,0xB    enderecos do guest no heartbeat
    --timeout S       encerra o jogo apos S segundos
    -h, --help        esta ajuda

PREFERENCIAS:
    <jogo>/launcher.cfg (chave = valor), gerado comentado na primeira execucao e editado pela
    janela (ps2launch-gui). Precedencia: linha de comando > ambiente (variaveis PS2X_* ja
    definidas) > launcher.cfg > game.toml.
";

struct Cli {
    command: String,
    game: PathBuf,
    iso: Option<PathBuf>,
    force: bool,
    deep: bool,
    run: run::RunOptions,
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next().ok_or_else(|| err!("{flag} precisa de um valor"))
}

fn parse(argv: Vec<String>) -> Result<Option<Cli>> {
    let mut it = argv.into_iter().peekable();
    let mut cli = Cli {
        command: String::new(),
        game: PathBuf::from("."),
        iso: None,
        force: false,
        deep: false,
        run: Default::default(),
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(None),
            "--game" => cli.game = next_value(&mut it, &a)?.into(),
            "--iso" => cli.iso = Some(next_value(&mut it, &a)?.into()),
            "--force" => cli.force = true,
            "--deep" => cli.deep = true,
            "--fps" => cli.run.fps = Some(next_value(&mut it, &a)?.parse().map_err(|_| err!("--fps espera um inteiro"))?),
            "--pad-script" => cli.run.pad_script = Some(next_value(&mut it, &a)?),
            "--fps-log" => cli.run.fps_log = Some(next_value(&mut it, &a)?),
            "--shot-dir" => cli.run.shot_dir = Some(next_value(&mut it, &a)?),
            "--missing-log" => cli.run.missing_log = Some(next_value(&mut it, &a)?),
            "--heartbeat" => cli.run.heartbeat = true,
            "--renderer" => cli.run.renderer = Some(options::validate_renderer(&next_value(&mut it, &a)?)?),
            "--internal-scale" => cli.run.internal_scale = Some(options::parse_scale(&next_value(&mut it, &a)?)?),
            "--crt" => {
                // Valor opcional: so consome o proximo argumento se for um nivel valido.
                let level = it.next_if(|n| options::Crt::parse(n).is_some());
                cli.run.crt = Some(level.map_or(Ok(options::Crt::Medio), |l| options::parse_crt(&l))?);
            }
            "--vfs-cache-mb" => cli.run.vfs_cache_mb = Some(options::parse_cache_mb(&next_value(&mut it, &a)?)?),
            "--widescreen" => cli.run.widescreen = true,
            "--vsync" => cli.run.vsync = Some(true),
            "--peek" => cli.run.peek = Some(next_value(&mut it, &a)?),
            "--timeout" => cli.run.timeout_secs = Some(next_value(&mut it, &a)?.parse().map_err(|_| err!("--timeout espera um numero"))?),
            "--" => {
                cli.run.extra_args = it.by_ref().collect();
            }
            s if s.starts_with('-') => return Err(err!("opcao desconhecida: {s} (veja --help)")),
            _ if cli.command.is_empty() => cli.command = a,
            _ => return Err(err!("argumento inesperado: {a}")),
        }
    }
    if cli.command.is_empty() {
        return Ok(None);
    }
    Ok(Some(cli))
}

fn execute(cli: Cli) -> Result<i32> {
    let game = std::path::absolute(&cli.game)?;
    let cfg = GameConfig::load(&game)?;
    log::init(&game.join("logs"));
    info!("jogo: {} ({})", cfg.name, cfg.serial);
    if cli.command == "info" {
        info!("work={} extrair={:?} executavel={:?}", cfg.work_dir.display(), cfg.extract_files, cfg.run_executable);
        info!("hardware: {}", hw::detect(true).summary());
        return Ok(0);
    }
    if cli.command == "hashes" {
        assets::print_known_table(&cfg)?;
        return Ok(0);
    }
    if !matches!(cli.command.as_str(), "verify" | "extract" | "run") {
        return Err(err!("comando desconhecido: {} (veja --help)", cli.command));
    }
    // launcher.cfg (gerado na primeira vez): flags vencem o cfg, que vence o padrao.
    let (settings, warnings) = Settings::load_or_create(&game)?;
    for w in warnings {
        warn!("{}: {w}", settings::FILE_NAME);
    }
    let mut cli = cli;
    settings.apply(&mut cli.run);
    let iso_choice = cli.iso.clone().or_else(|| settings.iso.clone());
    let iso = dump::find_iso(&cfg, iso_choice.as_deref())?;
    dump::verify(&cfg, &iso)?;
    dump::verify_sizes(&cfg, &iso)?;
    match cli.command.as_str() {
        "verify" => {
            if cli.deep {
                assets::verify_deep(&cfg)?;
                assets::check_known_hashes(&cfg)?;
            }
            Ok(0)
        }
        "extract" => {
            assets::extract(&cfg, &iso, cli.force)?;
            Ok(0)
        }
        _ => {
            assets::extract(&cfg, &iso, cli.force)?;
            report_hardware(&cli.run);
            run::run(&cfg, &cli.run)
        }
    }
}

/// Registra RAM/VRAM e avisa (sem bloquear) se a escala ou o cache pedirem mais que o disponivel.
fn report_hardware(o: &run::RunOptions) {
    let heavy = o.internal_scale.is_some_and(|s| s > 2) || o.vfs_cache_mb.is_some_and(|c| c >= options::CACHE_LARGE_MB);
    let h = hw::detect(heavy);
    info!("hardware: {}", h.summary());
    for w in h.warnings(o.internal_scale, o.vfs_cache_mb) {
        warn!("{w}");
    }
}

fn main() -> ExitCode {
    let cli = match parse(std::env::args().skip(1).collect()) {
        Ok(Some(c)) => c,
        Ok(None) => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("erro: {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    match execute(cli) {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(e) => {
            error!("{e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<Option<Cli>> {
        parse(a.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn parses_new_options() {
        let c = p(&["run", "--renderer", "HW", "--internal-scale", "4", "--crt", "forte", "--vfs-cache-mb", "0", "--widescreen"]).unwrap().unwrap();
        assert_eq!(c.run.renderer.as_deref(), Some("hw"));
        assert_eq!(c.run.internal_scale, Some(4));
        assert_eq!(c.run.crt, Some(options::Crt::Forte));
        assert_eq!(c.run.vfs_cache_mb, Some(0));
        assert!(c.run.widescreen);
    }

    #[test]
    fn crt_value_is_optional() {
        let c = p(&["--crt", "run"]).unwrap().unwrap();
        assert_eq!((c.run.crt, c.command.as_str()), (Some(options::Crt::Medio), "run"));
        assert!(p(&["run", "--crt"]).unwrap().unwrap().run.crt == Some(options::Crt::Medio));
    }

    #[test]
    fn rejects_bad_values() {
        assert!(p(&["run", "--renderer", "vulkan"]).is_err());
        assert!(p(&["run", "--internal-scale", "0"]).is_err());
        assert!(p(&["run", "--internal-scale", "9"]).is_err());
        assert!(p(&["run", "--vfs-cache-mb", "x"]).is_err());
    }
}
