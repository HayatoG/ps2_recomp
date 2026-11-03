//! Cores, fontes e tamanhos da janela. Fontes: as do Windows (Segoe UI), lidas de
//! `C:\Windows\Fonts` em tempo de execucao; sem elas, as padrao do egui. Nada de terceiros embutido.
use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke};
use std::sync::Arc;

/// Paleta: azul-petroleo/preto, acento ciano e um laranja quente discreto.
pub struct Palette {
    pub base: Color32,
    pub panel: Color32,
    pub panel_hi: Color32,
    pub line: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub warm: Color32,
    pub error: Color32,
    pub warn: Color32,
    pub disabled: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const P: Palette = Palette {
    base: rgb(0x0a1a20),
    panel: rgb(0x10272f),
    panel_hi: rgb(0x16343e),
    line: rgb(0x26464f),
    ink: rgb(0xe6f3f5),
    muted: rgb(0x9db6bd),
    accent: rgb(0x2fd0e0),
    accent_hover: rgb(0x6fe3ee),
    on_accent: rgb(0x03181c),
    warm: rgb(0xf0965a),
    error: rgb(0xff7a7a),
    warn: rgb(0xffc457),
    disabled: rgb(0x5e757c),
};

/// Tamanhos de texto (pontos). O minimo e 14 (legivel a 1280x800).
pub mod size {
    pub const TITLE: f32 = 34.0;
    pub const TITLE_MIN: f32 = 22.0;
    pub const STATUS: f32 = 15.0;
    pub const TAB: f32 = 19.0;
    pub const HEADING: f32 = 15.0;
    pub const BODY: f32 = 15.0;
    pub const NOTE: f32 = 14.0;
    pub const CARD_TITLE: f32 = 18.0;
    pub const BADGE: f32 = 14.0;
    pub const BUTTON: f32 = 15.0;
    pub const PLAY: f32 = 28.0;
    pub const CAPTION: f32 = 18.0;
}

const REGULAR: &str = "ps2l-regular";
const SEMIBOLD: &str = "ps2l-semibold";
const BOLD: &str = "ps2l-bold";
const TITLE: &str = "ps2l-title";
const SYMBOL: &str = "ps2l-symbol";

pub fn body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

/// Titulo: Segoe UI Black Italic (ou Bold Italic).
pub fn title(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(TITLE.into()))
}

fn system_font(names: &[&str]) -> Option<Vec<u8>> {
    let dir = std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(|| "C:\\Windows".into());
    names.iter().find_map(|n| std::fs::read(dir.join("Fonts").join(n)).ok())
}

/// Fontes: Segoe UI (regular, semibold, bold, black italic) + Segoe UI Symbol para ✕○□△, e as
/// padrao do egui como ultimo recurso. Cada familia cai para a anterior se o arquivo faltar.
pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut add = |name: &str, files: &[&str]| -> bool {
        match system_font(files) {
            Some(bytes) => {
                fonts.font_data.insert(name.into(), Arc::new(FontData::from_owned(bytes)));
                true
            }
            None => false,
        }
    };
    let has_regular = add(REGULAR, &["segoeui.ttf"]);
    let has_semibold = add(SEMIBOLD, &["seguisb.ttf", "segoeuib.ttf"]);
    let has_bold = add(BOLD, &["segoeuib.ttf", "seguisb.ttf"]);
    let has_title = add(TITLE, &["seguibli.ttf", "segoeuiz.ttf", "segoeuib.ttf"]);
    let has_symbol = add(SYMBOL, &["seguisym.ttf"]);
    let chain = |first: Option<&str>| -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        if let Some(f) = first {
            v.push(f.into());
        }
        if has_regular {
            v.push(REGULAR.into());
        }
        if has_symbol {
            v.push(SYMBOL.into());
        }
        v.extend(fallback.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, chain(None));
    fonts.families.insert(FontFamily::Name(SEMIBOLD.into()), chain(has_semibold.then_some(SEMIBOLD)));
    fonts.families.insert(FontFamily::Name(BOLD.into()), chain(has_bold.then_some(BOLD)));
    fonts.families.insert(FontFamily::Name(TITLE.into()), chain(has_title.then_some(TITLE)));
    fonts
}

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = P.base;
    v.window_fill = P.panel;
    v.window_stroke = Stroke::new(1.0, P.line);
    v.extreme_bg_color = P.panel;
    v.text_edit_bg_color = Some(P.panel);
    v.override_text_color = Some(P.ink);
    v.selection.bg_fill = P.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, P.accent);
    v.text_cursor.stroke = Stroke::new(2.0, P.accent);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.bg_stroke = Stroke::new(1.0, P.line);
        w.corner_radius = CornerRadius::same(6);
    }
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, P.muted);
    v.widgets.active.bg_stroke = Stroke::new(1.5, P.accent);
    ctx.all_styles_mut(move |style| {
        style.visuals = v.clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.text_styles.insert(egui::TextStyle::Body, body(size::BODY));
        style.text_styles.insert(egui::TextStyle::Button, body(size::BUTTON));
        style.text_styles.insert(egui::TextStyle::Small, body(size::NOTE));
    });
}
