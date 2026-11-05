//! A janela: estado, desenho das tres abas, rodape e navegacao por teclado/controle.
use super::art::{self, Art};
use super::pad::{self, Nav, Pad};
use super::strings::{self, fill, Strings};
use super::theme::{self, size, P};
use super::widgets::{self, Card, Chip};
use super::worker::{Jobs, Msg};
use crate::hw::{self, Hardware};
use crate::options::{Bands, Crt, Threads};
use crate::settings::{Profile, Renderer, Settings, DECK_FPS_CHOICES, FPS_CHOICES};
use crate::{assets, config::GameConfig, run};
use eframe::egui::{self, pos2, vec2, Color32, Id, Key, Rect, RichText, Sense};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Tab {
    Play,
    Picture,
    Controls,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Play, Tab::Picture, Tab::Controls];

    fn step(self, by: isize) -> Tab {
        let i = Tab::ALL.iter().position(|t| *t == self).unwrap_or(0) as isize;
        Tab::ALL[(i + by).rem_euclid(3) as usize]
    }
}

enum DumpState {
    Checking,
    Ok,
    Bad(String),
}

pub struct App {
    s: &'static Strings,
    cfg: GameConfig,
    settings: Settings,
    dirty: bool,
    iso_text: String,
    iso_explicit: bool,
    iso_found: Option<PathBuf>,
    iso_auto: bool,
    dump: DumpState,
    verify_gen: u64,
    edit_at: Option<Instant>,
    status: Option<assets::LocalStatus>,
    extracting: Option<assets::Progress>,
    launch_after_extract: bool,
    launching: bool,
    message: Option<(String, Color32)>,
    port_error: Option<String>,
    tab: Tab,
    art: Art,
    pad: Pad,
    pad_name: Option<String>,
    pad_checked: Option<Instant>,
    pending: Vec<Nav>,
    text_id: Option<Id>,
    jobs: Jobs,
    rx: Receiver<Msg>,
    /// RAM/VRAM, detectadas em segundo plano ao abrir a janela.
    hw: Arc<OnceLock<Hardware>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, cfg: GameConfig, settings: Settings) -> App {
        theme::install(&cc.egui_ctx);
        let (tx, rx) = std::sync::mpsc::channel();
        let jobs = Jobs { tx, ctx: cc.egui_ctx.clone() };
        let art = Art::load(&cc.egui_ctx, &cfg.root);
        let iso_text = settings.iso.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        let mut app = App {
            s: strings::get(),
            port_error: run::executable(&cfg).err().map(|e| e.0),
            iso_explicit: settings.iso.is_some(),
            cfg,
            settings,
            dirty: false,
            iso_text,
            iso_found: None,
            iso_auto: false,
            dump: DumpState::Checking,
            verify_gen: 0,
            edit_at: None,
            status: None,
            extracting: None,
            launch_after_extract: false,
            launching: false,
            message: None,
            tab: Tab::Play,
            art,
            pad: Pad::new(),
            pad_name: None,
            pad_checked: None,
            pending: Vec::new(),
            text_id: None,
            jobs,
            rx,
            hw: Arc::new(OnceLock::new()),
        };
        let slot = app.hw.clone();
        let ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            let _ = slot.set(hw::detect(true));
            ctx.request_repaint();
        });
        app.start_verify();
        app.jobs.status(app.cfg.clone());
        app
    }

    fn start_verify(&mut self) {
        self.verify_gen += 1;
        self.dump = DumpState::Checking;
        let explicit = self.iso_explicit.then(|| PathBuf::from(self.iso_text.trim().trim_matches('"'))).filter(|p| !p.as_os_str().is_empty());
        self.jobs.verify(self.cfg.clone(), explicit, self.verify_gen);
    }

    fn busy(&self) -> bool {
        self.extracting.is_some() || self.launching
    }

    /// Motivo para o JOGAR estar desabilitado (`None` = pronto).
    fn play_blocker(&self) -> Option<String> {
        if self.busy() {
            return Some(self.s.play_disabled_busy.into());
        }
        if let Some(e) = &self.port_error {
            return Some(fill(self.s.status_no_port, &[e]));
        }
        match self.dump {
            DumpState::Checking => Some(self.s.play_disabled_checking.into()),
            DumpState::Bad(_) => Some(self.s.play_disabled_dump.into()),
            DumpState::Ok => None,
        }
    }

    fn data_complete(&self) -> bool {
        self.status.as_ref().map(|s| s.complete()).unwrap_or(false)
    }

    // ------------------------------------------------------------------ acoes

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Verified { gen, .. } if gen != self.verify_gen => {}
                Msg::Verified { iso, auto, result, .. } => {
                    self.iso_auto = auto;
                    self.iso_found = iso.clone();
                    if auto && !self.iso_explicit {
                        self.iso_text = iso.map(|p| p.display().to_string()).unwrap_or_default();
                    }
                    self.dump = match result {
                        Ok(()) => DumpState::Ok,
                        Err(e) => DumpState::Bad(e),
                    };
                }
                Msg::Status(st) => self.status = Some(st),
                Msg::Progress(p) => self.extracting = Some(p),
                Msg::Extracted(r) => {
                    self.extracting = None;
                    match r {
                        Ok(()) => {
                            crate::info!("extracao concluida pela janela");
                            if std::mem::take(&mut self.launch_after_extract) {
                                self.launch();
                            } else {
                                self.message = Some((self.s.extract_done.into(), P.accent));
                            }
                        }
                        Err(e) => {
                            self.launch_after_extract = false;
                            self.message = Some((fill(self.s.extract_failed, &[&e]), P.error));
                        }
                    }
                }
                Msg::Launched(r) => {
                    self.launching = false;
                    match r {
                        Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(e) => self.message = Some((fill(self.s.launch_failed, &[&e]), P.error)),
                    }
                }
            }
        }
    }

    fn save(&mut self) -> bool {
        match self.settings.save(&self.cfg.root) {
            Ok(()) => {
                self.dirty = false;
                crate::info!("configuracoes salvas em {}", self.cfg.root.join(crate::settings::FILE_NAME).display());
                true
            }
            Err(e) => {
                crate::error!("{e}");
                self.message = Some((fill(self.s.save_failed, &[&e]), P.error));
                false
            }
        }
    }

    fn play(&mut self) {
        if self.play_blocker().is_some() {
            self.tab = Tab::Play;
            return;
        }
        if self.settings.profile.is_none() {
            self.settings.profile = Some(Profile::Recommended);
        }
        if !self.save() {
            return;
        }
        self.message = None;
        if self.data_complete() {
            self.launch();
        } else {
            self.launch_after_extract = true;
            self.extract();
        }
    }

    fn extract(&mut self) {
        let Some(iso) = self.iso_found.clone() else { return };
        if self.busy() || !matches!(self.dump, DumpState::Ok) {
            return;
        }
        self.message = None;
        self.extracting = Some(assets::Progress::default());
        self.jobs.extract(self.cfg.clone(), iso);
    }

    fn launch(&mut self) {
        let mut opts = run::RunOptions { no_console: true, ..Default::default() };
        self.settings.apply(&mut opts);
        self.launching = true;
        self.jobs.launch(self.cfg.clone(), opts);
    }

    fn export_logs(&mut self) {
        let logs = self.cfg.root.join("logs");
        let files: Vec<PathBuf> = std::fs::read_dir(&logs)
            .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file() && !p.ends_with(".gitkeep")).collect())
            .unwrap_or_default();
        if files.is_empty() {
            self.message = Some((self.s.no_logs.into(), P.warn));
            return;
        }
        let Some(dest) = rfd::FileDialog::new().set_title(self.s.export_title).pick_folder() else { return };
        let out = dest.join(format!("ps2launch-logs-{}", crate::log::stamp()));
        let r = std::fs::create_dir_all(&out).and_then(|_| {
            for f in files.iter().chain(std::iter::once(&self.cfg.root.join(crate::settings::FILE_NAME))) {
                if f.is_file() {
                    std::fs::copy(f, out.join(f.file_name().unwrap_or_default()))?;
                }
            }
            Ok(())
        });
        self.message = Some(match r {
            Ok(()) => {
                crate::info!("logs exportados para {}", out.display());
                (fill(self.s.exported, &[&out.display()]), P.accent)
            }
            Err(e) => (e.to_string(), P.error),
        });
    }

    fn open_folder(&self) {
        if let Err(e) = std::process::Command::new("explorer").arg(&self.cfg.root).spawn() {
            crate::warn!("nao consegui abrir a pasta: {e}");
        }
    }

    /// Perfil efetivo e os valores que ele fixa (para mostrar na aba Imagem).
    fn effective(&self) -> (Profile, Renderer, u32, bool) {
        let r = self.settings.resolve();
        (
            self.settings.profile.unwrap_or(Profile::Recommended),
            r.renderer.unwrap_or(Renderer::Cpu),
            r.fps.unwrap_or(60),
            r.vsync.unwrap_or(false),
        )
    }

    /// Editar uma opcao da aba Imagem passa o perfil para Personalizado, mantendo o que se via.
    fn make_custom(&mut self) {
        let r = self.settings.resolve();
        let s = &mut self.settings;
        s.profile = Some(Profile::Custom);
        s.renderer = r.renderer.or(s.renderer);
        s.fps = r.fps.or(s.fps);
        s.vsync = r.vsync.or(s.vsync);
        s.internal_scale = r.internal_scale;
        s.crt = r.crt;
        s.crt_strength = r.crt_strength;
        s.bands = r.bands;
        s.queue_kb = r.queue_kb;
        self.dirty = true;
    }

    // ------------------------------------------------------------------ desenho

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width() - 90.0;
        let name = self.cfg.name.clone();
        let one_line = [size::TITLE, 32.0, 30.0, 28.0, 26.0]
            .into_iter()
            .find(|&s| ui.painter().layout_no_wrap(name.clone(), theme::title(s), P.ink).size().x <= w);
        let g = match one_line {
            Some(s) => ui.painter().layout_no_wrap(name, theme::title(s), P.ink),
            None => ui.painter().layout(name, theme::title(size::TITLE_MIN + 6.0), P.ink, w),
        };
        let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
        ui.painter().galley(r.min, g, P.ink);
        let (text, color) = self.status_line();
        ui.add_space(-4.0);
        ui.add(egui::Label::new(RichText::new(text).font(theme::body(size::STATUS)).color(color)).wrap());
        ui.add_space(6.0);
    }

    fn status_line(&self) -> (String, Color32) {
        let s = self.s;
        if let Some((m, c)) = &self.message {
            return (m.clone(), *c);
        }
        if let Some(p) = &self.extracting {
            let pct = if p.total > 0 { p.done * 100 / p.total } else { 0 };
            return (fill(s.status_extracting, &[&pct]), P.accent);
        }
        if self.launching {
            return (s.status_launching.into(), P.accent);
        }
        if let Some(e) = &self.port_error {
            return (fill(s.status_no_port, &[e]), P.warn);
        }
        match &self.dump {
            DumpState::Checking => (s.status_checking.into(), P.muted),
            DumpState::Bad(_) if self.iso_found.is_none() => (s.status_need_dump.into(), P.warn),
            DumpState::Bad(_) => (s.status_bad_dump.into(), P.error),
            DumpState::Ok => match &self.status {
                Some(st) if st.total > 0 && !st.complete() => {
                    (fill(s.status_need_extract, &[&gb(st.bytes_total - st.bytes_present)]), P.warn)
                }
                _ => (s.status_ready.into(), P.ink),
            },
        }
    }

    fn draw_play(&mut self, ui: &mut egui::Ui) {
        let s = self.s;
        widgets::heading(ui, s.disc_heading);
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut self.iso_text)
                .hint_text(RichText::new(s.disc_hint).color(P.muted))
                .font(theme::body(size::BODY))
                .margin(vec2(10.0, 7.0))
                .desired_width(ui.available_width() - 118.0);
            let r = ui.add(edit);
            self.text_id = Some(r.id);
            if r.changed() {
                self.iso_explicit = !self.iso_text.trim().is_empty();
                self.edit_at = Some(Instant::now());
            }
            if r.lost_focus() && self.edit_at.is_some() {
                self.edit_at = Some(Instant::now() - Duration::from_secs(1));
            }
            if widgets::outline_button(ui, s.browse, true).clicked() {
                let mut dlg = rfd::FileDialog::new().set_title(s.browse_title).add_filter(s.disc_filter, &["iso", "bin", "img"]);
                if let Some(dir) = self.iso_found.as_ref().and_then(|p| p.parent()).or(Some(self.cfg.root.as_path())) {
                    dlg = dlg.set_directory(dir);
                }
                if let Some(p) = dlg.pick_file() {
                    self.iso_text = p.display().to_string();
                    self.iso_explicit = true;
                    self.edit_at = Some(Instant::now() - Duration::from_secs(1));
                }
            }
        });
        match &self.dump {
            DumpState::Checking => {
                widgets::note(ui, s.checking_disc, P.muted);
            }
            DumpState::Ok => {
                let region = strings::region_name(&self.cfg.region);
                let r = widgets::note(ui, &fill(s.found, &[&self.cfg.serial, &region]), P.accent);
                if self.iso_auto {
                    if let Some(name) = self.iso_found.as_ref().and_then(|p| p.file_name()) {
                        r.on_hover_text(fill(s.found_auto, &[&name.to_string_lossy()]));
                    }
                }
            }
            DumpState::Bad(e) => {
                widgets::note(ui, e, if self.iso_found.is_none() { P.warn } else { P.error });
            }
        }
        if let Some(e) = &self.port_error {
            widgets::note(ui, e, P.warn);
        }
        ui.add_space(8.0);
        self.draw_profiles(ui);
        ui.add_space(8.0);
        self.draw_data(ui);
    }

    fn draw_profiles(&mut self, ui: &mut egui::Ui) {
        let s = self.s;
        let (profile, ..) = self.effective();
        ui.horizontal(|ui| {
            widgets::heading(ui, s.profile_heading);
            if profile == Profile::Deck {
                ui.add_space(24.0);
                ui.label(RichText::new(s.deck_fps_label).font(theme::body(size::NOTE)).color(P.muted));
                let current = self.settings.deck_fps.unwrap_or(60);
                let labels: Vec<String> = DECK_FPS_CHOICES.iter().map(|f| format!("{f} fps")).collect();
                let opts: Vec<Chip<u32>> =
                    DECK_FPS_CHOICES.iter().zip(&labels).map(|(v, l)| Chip { value: *v, label: l, badge: None, disabled: None }).collect();
                if let Some(v) = widgets::chips(ui, &opts, current) {
                    self.settings.deck_fps = Some(v);
                    self.dirty = true;
                }
            }
        });
        ui.add_space(10.0);
        let cards = [
            (Profile::Original, s.profile_original, s.profile_original_note, None),
            (Profile::Recommended, s.profile_recommended, s.profile_recommended_note, Some(s.badge_default)),
            (Profile::Deck, s.profile_deck, s.profile_deck_note, None),
            (Profile::Custom, s.profile_custom, s.profile_custom_note, None),
        ];
        let gap = 10.0;
        let w = ((ui.available_width() - 3.0 * gap) / 4.0).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (p, title, note, badge) in cards {
                let c = Card { title, note, badge, selected: profile == p };
                if widgets::card(ui, vec2(w, 108.0), &c).clicked() && profile != p {
                    if p == Profile::Custom {
                        self.make_custom();
                    } else {
                        self.settings.profile = Some(p);
                        self.dirty = true;
                    }
                }
            }
        });
    }

    fn draw_data(&mut self, ui: &mut egui::Ui) {
        let s = self.s;
        widgets::heading(ui, s.data_heading);
        let (text, color) = match &self.status {
            None => (s.checking_disc.to_string(), P.muted),
            Some(st) if st.total == 0 => (s.data_none.to_string(), P.muted),
            Some(st) => {
                let mut t = fill(s.data_status, &[&st.present, &st.total, &gb(st.bytes_present), &gb(st.bytes_total)]);
                if let Some(free) = st.free {
                    t.push_str(" · ");
                    t.push_str(&fill(s.data_free, &[&gb(free)]));
                }
                if st.complete() {
                    t.push_str(" · ");
                    t.push_str(s.data_complete);
                }
                (t, if st.complete() { P.ink } else { P.warn })
            }
        };
        ui.horizontal(|ui| {
            let can = !self.busy() && matches!(self.dump, DumpState::Ok) && !self.data_complete();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::outline_button(ui, s.extract, can).clicked() {
                    self.extract();
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    widgets::note(ui, &text, color);
                });
            });
        });
        if let Some(p) = &self.extracting {
            let frac = if p.total > 0 { p.done as f32 / p.total as f32 } else { 0.0 };
            widgets::progress_bar(ui, frac);
            widgets::note(ui, &fill(s.extracting_file, &[&p.file, &(p.done >> 20), &(p.total >> 20)]), P.muted);
        }
    }

    fn draw_picture(&mut self, ui: &mut egui::Ui) {
        let s = self.s;
        let (profile, renderer, fps, vsync) = self.effective();
        let r = self.settings.resolve();
        let rkey = renderer.key();
        let is_hw = crate::options::is_hw(rkey);
        let uses_bands = crate::options::uses_bands(rkey);
        if profile != Profile::Custom {
            widgets::note(ui, s.picture_profile_note, P.muted);
            ui.add_space(2.0);
        }
        // --- renderizador
        widgets::heading(ui, s.renderer_heading);
        let main = [
            Chip { value: Renderer::Hw, label: s.renderer_hw, badge: Some(s.badge_fastest), disabled: None },
            Chip { value: Renderer::CpuThread, label: s.renderer_cpu_thread, badge: None, disabled: None },
            Chip { value: Renderer::Cpu, label: s.renderer_cpu, badge: None, disabled: None },
        ];
        let mut pick = widgets::chips(ui, &main, renderer);
        let advanced = [Renderer::D3d11, Renderer::HwSerial, Renderer::CpuBands].contains(&renderer);
        egui::CollapsingHeader::new(RichText::new(s.advanced_heading).font(theme::semibold(size::BODY)).color(P.muted))
            .id_salt("ps2l-adv")
            .default_open(advanced)
            .show(ui, |ui| {
                let adv = [
                    Chip { value: Renderer::D3d11, label: s.renderer_d3d11, badge: None, disabled: None },
                    Chip { value: Renderer::HwSerial, label: s.renderer_hw_serial, badge: None, disabled: None },
                    Chip { value: Renderer::CpuBands, label: s.renderer_cpu_bands, badge: None, disabled: None },
                ];
                pick = pick.or(widgets::chips(ui, &adv, renderer));
            });
        if let Some(r) = pick {
            self.make_custom();
            self.settings.renderer = Some(r);
        }
        let (note, color) = match renderer {
            Renderer::Hw => (s.note_hw, P.muted),
            Renderer::CpuThread => (s.note_cpu_thread, P.muted),
            Renderer::Cpu => (s.note_cpu, P.muted),
            Renderer::CpuBands => (s.note_cpu_bands, P.muted),
            Renderer::HwSerial => (s.note_hw_serial, P.muted),
            Renderer::D3d11 => (s.d3d11_warning, P.warn),
        };
        widgets::note(ui, note, color);

        // --- resolucao interna (so hw)
        widgets::heading(ui, s.scale_heading);
        let scale = r.internal_scale.unwrap_or(1);
        let why = (!is_hw).then_some(s.scale_only_hw);
        let labels: Vec<String> = (1..=8u32).map(|n| format!("{n}x")).collect();
        let opts: Vec<Chip<u32>> = (1..=8u32).zip(&labels).map(|(v, l)| Chip { value: v, label: l, badge: None, disabled: why }).collect();
        if let Some(v) = widgets::chips(ui, &opts, scale) {
            self.make_custom();
            self.settings.internal_scale = Some(v);
        }
        widgets::note(ui, s.scale_note, P.muted);
        match self.hw.get() {
            Some(h) => {
                if is_hw {
                    if let Some(v) = h.vram.filter(|v| *v >= 6 << 30) {
                        if scale <= 1 {
                            widgets::note(ui, &fill(s.scale_recommend, &[&gb(v)]), P.accent);
                        }
                    }
                    for line in h.warnings(Some(scale), self.settings.vfs_cache_mb) {
                        let msg = if line.starts_with("VRAM") { s.scale_warn_vram } else { s.scale_warn_ram };
                        widgets::note(ui, msg, P.warn);
                    }
                }
                widgets::note(ui, &h.summary(), P.muted);
            }
            None => {
                widgets::note(ui, s.hw_detecting, P.muted);
            }
        }

        // --- CRT
        widgets::heading(ui, s.crt_heading);
        let crt = r.crt.unwrap_or(Crt::Off);
        let labels = [s.crt_off, s.crt_suave, s.crt_medio, s.crt_forte];
        let opts: Vec<Chip<Crt>> = Crt::ALL.iter().zip(labels).map(|(c, l)| Chip { value: *c, label: l, badge: None, disabled: None }).collect();
        if let Some(v) = widgets::chips(ui, &opts, crt) {
            self.make_custom();
            self.settings.crt = Some(v);
        }
        if crt != Crt::Off {
            ui.label(RichText::new(s.crt_strength_label).font(theme::body(size::NOTE)).color(P.muted));
            let labels: Vec<String> = [25u32, 50, 75, 100].iter().map(|n| format!("{n}%")).collect();
            let mut opts = vec![Chip { value: None, label: s.crt_strength_level, badge: None, disabled: None }];
            opts.extend([25u32, 50, 75, 100].iter().zip(&labels).map(|(v, l)| Chip { value: Some(*v), label: l, badge: None, disabled: None }));
            if let Some(v) = widgets::chips(ui, &opts, r.crt_strength) {
                self.make_custom();
                self.settings.crt_strength = v;
            }
        }
        widgets::note(ui, s.crt_note, P.muted);

        // --- cache de arquivos (independe do perfil)
        widgets::heading(ui, s.cache_heading);
        let mut opts = vec![
            Chip { value: None, label: s.cache_auto, badge: None, disabled: None },
            Chip { value: Some(0u32), label: s.cache_off, badge: None, disabled: None },
        ];
        let labels: Vec<String> = [256u32, 512, 1024, 2048].iter().map(|n| format!("{n} MB")).collect();
        opts.extend([256u32, 512, 1024, 2048].iter().zip(&labels).map(|(v, l)| Chip { value: Some(*v), label: l, badge: None, disabled: None }));
        if let Some(v) = widgets::chips(ui, &opts, self.settings.vfs_cache_mb) {
            self.settings.vfs_cache_mb = v;
            self.dirty = true;
        }
        widgets::note(ui, s.cache_note, P.muted);

        // --- rasterizacao em faixas (so cpu-thread / cpu-bands)
        widgets::heading(ui, s.bands_heading);
        let why = (!uses_bands).then_some(s.bands_only);
        let labels: Vec<String> = [2u32, 4, 8].iter().map(|n| n.to_string()).collect();
        let mut opts = vec![
            Chip { value: Bands::Auto, label: s.bands_auto, badge: None, disabled: why },
            Chip { value: Bands::Off, label: s.bands_off, badge: None, disabled: why },
        ];
        opts.extend([2u32, 4, 8].iter().zip(&labels).map(|(v, l)| Chip { value: Bands::N(*v), label: l, badge: None, disabled: why }));
        if let Some(v) = widgets::chips(ui, &opts, r.bands.unwrap_or(Bands::Auto)) {
            self.make_custom();
            self.settings.bands = Some(v);
        }

        // --- fila do GS
        widgets::heading(ui, s.queue_heading);
        let opts = [
            Chip { value: None, label: s.queue_default, badge: None, disabled: None },
            Chip { value: Some(8192u32), label: "8192 KB", badge: None, disabled: None },
            Chip { value: Some(16384u32), label: "16384 KB", badge: None, disabled: None },
        ];
        if let Some(v) = widgets::chips(ui, &opts, r.queue_kb) {
            self.make_custom();
            self.settings.queue_kb = v;
        }
        widgets::note(ui, s.queue_note, P.muted);

        // --- FPS, vsync, diagnostico
        widgets::heading(ui, s.fps_heading);
        let labels: Vec<String> = FPS_CHOICES.iter().map(|f| if *f == 0 { s.fps_unlimited.to_string() } else { f.to_string() }).collect();
        let opts: Vec<Chip<u32>> = FPS_CHOICES.iter().zip(&labels).map(|(v, l)| Chip { value: *v, label: l, badge: None, disabled: None }).collect();
        if let Some(v) = widgets::chips(ui, &opts, fps) {
            self.make_custom();
            self.settings.fps = Some(v);
        }
        if fps == 0 {
            widgets::note(ui, s.fps_unlimited_note, P.muted);
        }
        ui.add_space(6.0);
        let mut v = vsync;
        let why = (!is_hw && renderer != Renderer::D3d11).then_some(s.vsync_cpu_reason);
        if widgets::toggle(ui, &mut v, s.vsync_label, why).on_hover_text(s.vsync_note).changed() {
            self.make_custom();
            self.settings.vsync = Some(v);
        }
        let mut hb = self.settings.heartbeat.unwrap_or(false);
        if widgets::toggle(ui, &mut hb, s.heartbeat_label, None).on_hover_text(s.heartbeat_note).changed() {
            self.settings.heartbeat = Some(hb);
            self.dirty = true;
        }
        // --- opcoes avancadas / experimentais (sempre guardadas no cfg; so exportadas quando valem)
        let adv_open = r.hw_cpuframe || r.threads.is_some_and(|t| t != Threads::Off) || r.hw_workers.is_some();
        egui::CollapsingHeader::new(RichText::new(s.adv_opts_heading).font(theme::semibold(size::BODY)).color(P.muted))
            .id_salt("ps2l-adv-opts")
            .default_open(adv_open)
            .show(ui, |ui| {
                let mut cf = r.hw_cpuframe;
                let why = (!is_hw).then_some(s.cpuframe_hw_only);
                if widgets::toggle(ui, &mut cf, s.cpuframe_label, why).on_hover_text(s.cpuframe_note).changed() {
                    self.settings.hw_cpuframe = Some(cf);
                    self.dirty = true;
                }
                widgets::note(ui, s.cpuframe_note, P.warm);
                widgets::heading(ui, s.threads_heading);
                let opts: Vec<Chip<Threads>> = Threads::ALL
                    .iter()
                    .map(|t| Chip {
                        value: *t,
                        label: if *t == Threads::Off { s.threads_off } else { t.key() },
                        badge: (*t != Threads::Off).then_some(s.badge_experimental),
                        disabled: None,
                    })
                    .collect();
                if let Some(v) = widgets::chips(ui, &opts, r.threads.unwrap_or(Threads::Off)) {
                    self.settings.threads = Some(v);
                    self.dirty = true;
                }
                widgets::note(ui, s.threads_note, P.warm);
                widgets::heading(ui, s.hw_workers_heading);
                let why = (!is_hw).then_some(s.hw_workers_only);
                let labels: Vec<String> = [2u32, 4, 8].iter().map(|n| n.to_string()).collect();
                let mut opts = vec![
                    Chip { value: None, label: s.hw_workers_auto, badge: None, disabled: why },
                    Chip { value: Some(0u32), label: s.hw_workers_off, badge: None, disabled: why },
                ];
                opts.extend([2u32, 4, 8].iter().zip(&labels).map(|(v, l)| Chip { value: Some(*v), label: l, badge: None, disabled: why }));
                if let Some(v) = widgets::chips(ui, &opts, r.hw_workers) {
                    self.settings.hw_workers = v;
                    self.dirty = true;
                }
            });
        widgets::heading(ui, s.more_heading);
        let soon = [
            Chip { value: 0u8, label: s.widescreen_label, badge: Some(s.badge_planned), disabled: Some(s.widescreen_reason) },
            Chip { value: 1u8, label: s.fullscreen, badge: Some(s.badge_soon), disabled: Some(s.soon_reason) },
        ];
        widgets::chips(ui, &soon, u8::MAX);
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        let s = self.s;
        egui::Grid::new("ps2l-controls").num_columns(3).striped(true).spacing([28.0, 7.0]).show(ui, |ui| {
            for h in [s.controls_col_ps2, s.controls_col_keyboard, s.controls_col_pad] {
                ui.label(RichText::new(h).font(theme::semibold(size::HEADING)).color(P.muted));
            }
            ui.end_row();
            for row in s.controls_rows {
                ui.label(RichText::new(row[0]).font(theme::semibold(size::BODY)).color(P.ink));
                ui.label(RichText::new(row[1]).font(theme::body(size::BODY)));
                ui.label(RichText::new(row[2]).font(theme::body(size::BODY)));
                ui.end_row();
            }
        });
        ui.add_space(8.0);
        widgets::note(ui, s.controls_note_sticks, P.muted);
        widgets::note(ui, s.controls_note_pad_wins, P.warm);
        match &self.pad_name {
            Some(n) => widgets::note(ui, &fill(s.pad_detected, &[n]), P.accent),
            None => widgets::note(ui, s.pad_none, P.muted),
        };
        ui.add_space(4.0);
        widgets::note(ui, s.controls_launcher_nav, P.muted);
    }

    fn draw_footer(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let s = self.s;
        let plate = Rect::from_min_size(pos2(rect.right() - 196.0, rect.center().y - 28.0), vec2(196.0, 56.0));
        let blocker = self.play_blocker();
        let resp = widgets::play_plate(ui, plate, s.play, blocker.is_none());
        let resp = match &blocker {
            Some(why) => resp.on_hover_text(why.as_str()),
            None => resp,
        };
        if resp.clicked() {
            self.play();
        }
        let left = Rect::from_min_max(rect.min, pos2(plate.left() - 24.0, rect.bottom()));
        let hint = match &blocker {
            Some(why) => why.clone(),
            None if !self.data_complete() => s.hint_extract_first.into(),
            None if self.effective().1 == Renderer::D3d11 => format!("{} {}", s.hint_ready, s.hint_d3d11),
            None => s.hint_ready.into(),
        };
        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
            ui.add(egui::Label::new(RichText::new(hint).font(theme::body(size::NOTE)).color(P.muted)).wrap());
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 18.0;
                    if widgets::link_button(ui, s.export_logs).clicked() {
                        self.export_logs();
                    }
                    if widgets::link_button(ui, s.open_folder).clicked() {
                        self.open_folder();
                    }
                });
            });
        });
    }
}

fn gb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1u64 << 30) as f64).replace('.', ",")
}

impl eframe::App for App {
    /// Teclado e controle antes do egui: Ctrl+Tab/Q/E trocam de aba, Esc fecha, Enter sem foco
    /// joga; o controle vira teclas (ver `pad`).
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let focused = ctx.memory(|m| m.focused());
        let typing = focused.is_some() && focused == self.text_id;
        let mut close = false;
        raw.events.retain(|e| {
            let egui::Event::Key { key, pressed: true, modifiers, .. } = e else { return true };
            match key {
                Key::Tab if modifiers.ctrl => {
                    self.pending.push(if modifiers.shift { Nav::PrevTab } else { Nav::NextTab });
                    false
                }
                Key::Q | Key::E if !typing && !modifiers.any() => {
                    self.pending.push(if *key == Key::E { Nav::NextTab } else { Nav::PrevTab });
                    false
                }
                Key::Escape if !typing => {
                    close = true;
                    false
                }
                Key::Enter if focused.is_none() => {
                    self.pending.push(Nav::Play);
                    false
                }
                _ => true,
            }
        });
        // Solta de Ctrl+Tab/Q/E tambem e removida para nao vazar.
        raw.events.retain(|e| {
            !matches!(e, egui::Event::Key { key: Key::Tab, pressed: false, modifiers, .. } if modifiers.ctrl)
        });
        if close {
            if self.dirty {
                self.save();
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let navs = self.pad.poll();
        pad::inject(raw, &navs, typing, focused.is_none());
        self.pending.extend(navs.into_iter().filter(|n| matches!(n, Nav::Back | Nav::NextTab | Nav::PrevTab | Nav::Play)));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        if self.pad_checked.map(|t| t.elapsed() > Duration::from_secs(1)).unwrap_or(true) {
            self.pad_name = self.pad.connected();
            self.pad_checked = Some(Instant::now());
        }
        // Caminho digitado: verifica de novo apos uma pausa.
        if let Some(t) = self.edit_at {
            if t.elapsed() > Duration::from_millis(600) {
                self.edit_at = None;
                self.settings.iso = self.iso_explicit.then(|| PathBuf::from(self.iso_text.trim().trim_matches('"')));
                self.dirty = true;
                self.message = None;
                self.start_verify();
            }
        }
        for nav in std::mem::take(&mut self.pending) {
            match nav {
                Nav::Play => self.play(),
                Nav::NextTab => self.tab = self.tab.step(1),
                Nav::PrevTab => self.tab = self.tab.step(-1),
                Nav::Back if self.tab != Tab::Play => self.tab = Tab::Play,
                Nav::Back => ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id)
                    }
                }),
                _ => {}
            }
        }
        // Fechar pelo X (ou Alt+F4) tambem guarda o que mudou.
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty {
            self.save();
        }

        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, P.base);
        let strip = Rect::from_min_size(full.min, vec2((full.width() * 0.34).round(), full.height()));
        let caption = fill(self.s.caption_line, &[&self.cfg.serial, &strings::region_name(&self.cfg.region)]);
        art::draw_strip(ui.painter(), strip, &self.art, self.cfg.art_focus, &self.cfg.name, &caption);
        widgets::stripes(ui.painter(), full);

        let content = Rect::from_min_max(pos2(strip.right() + 20.0, full.top() + 20.0), pos2(full.right() - 30.0, full.bottom() - 18.0));
        let footer = Rect::from_min_max(pos2(content.left(), content.bottom() - 66.0), content.max);
        let body = Rect::from_min_max(content.min, pos2(content.right(), footer.top() - 14.0));
        ui.painter().hline(body.x_range(), footer.top() - 7.0, egui::Stroke::new(1.0, P.line));

        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            self.draw_header(ui);
            let names = [self.s.tab_play, self.s.tab_picture, self.s.tab_controls];
            let at = Tab::ALL.iter().position(|t| *t == self.tab).unwrap_or(0);
            if let Some(i) = widgets::tabs_bar(ui, &names, at) {
                self.tab = Tab::ALL[i];
            }
            ui.add_space(6.0);
            egui::ScrollArea::vertical().id_salt(self.tab).auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(4.0);
                match self.tab {
                    Tab::Play => self.draw_play(ui),
                    Tab::Picture => self.draw_picture(ui),
                    Tab::Controls => self.draw_controls(ui),
                }
                ui.add_space(4.0);
            });
        });
        self.draw_footer(ui, footer);

        // O controle nao gera eventos de janela: olhar de novo em breve.
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
