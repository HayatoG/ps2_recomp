//! Tarefas em segundo plano (a janela nunca trava): verificar o dump, medir os dados extraidos,
//! extrair e iniciar o port. Cada tarefa manda [`Msg`] por um canal e pede um redesenho.
use crate::{assets, config::GameConfig, dump, run};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

pub enum Msg {
    /// Resultado da verificacao numero `gen` (as antigas sao ignoradas).
    Verified { gen: u64, iso: Option<PathBuf>, auto: bool, result: Result<(), String> },
    Status(assets::LocalStatus),
    Progress(assets::Progress),
    Extracted(Result<(), String>),
    Launched(Result<(), String>),
}

#[derive(Clone)]
pub struct Jobs {
    pub tx: Sender<Msg>,
    pub ctx: egui::Context,
}

impl Jobs {
    fn send(&self, m: Msg) {
        let _ = self.tx.send(m);
        self.ctx.request_repaint();
    }

    /// Localiza (explicito ou pelo `iso_glob`) e valida o dump: serial, hash do ELF e tamanhos.
    pub fn verify(&self, cfg: GameConfig, explicit: Option<PathBuf>, gen: u64) {
        let me = self.clone();
        std::thread::spawn(move || {
            let auto = explicit.is_none();
            let (iso, result) = match dump::find_iso(&cfg, explicit.as_deref()) {
                Err(e) => (explicit, Err(e.0)),
                Ok(iso) => {
                    let r = dump::verify(&cfg, &iso).and_then(|_| dump::verify_sizes(&cfg, &iso)).map_err(|e| e.0);
                    if let Err(e) = &r {
                        crate::warn!("dump recusado: {e}");
                    }
                    (Some(iso), r)
                }
            };
            me.send(Msg::Verified { gen, iso, auto, result });
        });
    }

    pub fn status(&self, cfg: GameConfig) {
        let me = self.clone();
        std::thread::spawn(move || me.send(Msg::Status(assets::local_status(&cfg))));
    }

    pub fn extract(&self, cfg: GameConfig, iso: PathBuf) {
        let me = self.clone();
        std::thread::spawn(move || {
            let mut last = 0u64;
            let r = assets::extract_with(&cfg, &iso, false, &mut |p| {
                // ~1 mensagem a cada 8 MiB basta para a barra.
                if p.done < last || p.done - last >= 8 << 20 || p.done == p.total {
                    last = p.done;
                    me.send(Msg::Progress(p.clone()));
                }
            });
            if let Err(e) = &r {
                crate::error!("extracao: {e}");
            }
            me.send(Msg::Extracted(r.map_err(|e| e.0)));
            me.send(Msg::Status(assets::local_status(&cfg)));
        });
    }

    /// Inicia o port desacoplado e espera um pouco para pegar falha imediata.
    pub fn launch(&self, cfg: GameConfig, opts: run::RunOptions) {
        let me = self.clone();
        std::thread::spawn(move || {
            let r = run::spawn(&cfg, &opts).and_then(|mut s| run::check_started(&mut s, Duration::from_millis(1500)).map(|_| ()));
            if let Err(e) = &r {
                crate::error!("{e}");
            }
            me.send(Msg::Launched(r.map_err(|e| e.0)));
        });
    }
}
