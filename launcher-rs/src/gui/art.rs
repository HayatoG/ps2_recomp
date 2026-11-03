//! Arte do jogo em `<jogo>/artwork/` (fornecida pelo usuario; nunca embutida nem baixada).
//! `capsule_vertical.png` (ou `hero.png`) preenche a faixa esquerda com corte "cover";
//! `icon.png` vira o icone da janela; `logo.png` aparece no lugar gerado quando falta a capa.
use super::theme::{self, size, P};
use super::widgets;
use eframe::egui::{self, pos2, vec2, Color32, ColorImage, Painter, Rect, TextureHandle, Vec2};
use std::path::Path;

const STRIP_FILES: [&str; 2] = ["capsule_vertical.png", "hero.png"];
/// Lado maximo da textura (a faixa tem ~300x640 pontos; 2x cobre telas de alta densidade).
const MAX_SIDE: u32 = 1400;

fn decode(path: &Path, max_side: u32) -> Option<image::RgbaImage> {
    let img = image::open(path).ok()?;
    let img = if img.width().max(img.height()) > max_side { img.thumbnail(max_side, max_side) } else { img };
    Some(img.to_rgba8())
}

fn to_color_image(img: &image::RgbaImage) -> ColorImage {
    ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw())
}

pub struct Art {
    pub strip: Option<TextureHandle>,
    pub logo: Option<TextureHandle>,
}

impl Art {
    pub fn load(ctx: &egui::Context, game_dir: &Path) -> Art {
        let dir = game_dir.join("artwork");
        let strip = STRIP_FILES.iter().find_map(|f| decode(&dir.join(f), MAX_SIDE));
        let logo = if strip.is_none() { decode(&dir.join("logo.png"), 512) } else { None };
        let tex = |name: &str, img: Option<image::RgbaImage>| {
            img.map(|i| ctx.load_texture(name, to_color_image(&i), egui::TextureOptions::LINEAR))
        };
        Art { strip: tex("ps2l-strip", strip), logo: tex("ps2l-logo", logo) }
    }
}

/// `icon.png` em 256x256 para a janela.
pub fn window_icon(game_dir: &Path) -> Option<egui::IconData> {
    let img = image::open(game_dir.join("artwork").join("icon.png")).ok()?;
    let img = img.resize(256, 256, image::imageops::FilterType::Triangle).to_rgba8();
    Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}

/// Retangulo de UV que cobre `rect` com a imagem sem distorcer, centrado em `focus` (0..1).
pub fn cover_uv(image: Vec2, rect: Vec2, focus: Vec2) -> Rect {
    if image.x <= 0.0 || image.y <= 0.0 || rect.x <= 0.0 || rect.y <= 0.0 {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let scale = (rect.x / image.x).max(rect.y / image.y);
    let span = vec2((rect.x / (image.x * scale)).min(1.0), (rect.y / (image.y * scale)).min(1.0));
    let start = |f: f32, s: f32| (f - s / 2.0).clamp(0.0, 1.0 - s);
    let min = pos2(start(focus.x, span.x), start(focus.y, span.y));
    Rect::from_min_size(min, span)
}

/// Faixa esquerda: arte (ou placeholder), degrade fundindo no painel e legenda no rodape.
/// `focus` = ponto da arte (0..1) que fica no centro da faixa (`[art] focus` no game.toml).
pub fn draw_strip(painter: &Painter, rect: Rect, art: &Art, focus: [f32; 2], name: &str, caption: &str) {
    match &art.strip {
        Some(tex) => {
            let uv = cover_uv(tex.size_vec2(), rect.size(), vec2(focus[0], focus[1]));
            painter.image(tex.id(), rect, uv, Color32::WHITE);
        }
        None => placeholder(painter, rect, art, name),
    }
    // Funde a direita no painel e escurece o rodape para a legenda.
    widgets::gradient(painter, Rect::from_min_max(pos2(rect.left() + rect.width() * 0.5, rect.top()), rect.max), P.base, false);
    widgets::gradient(painter, Rect::from_min_max(pos2(rect.left(), rect.bottom() - 190.0), rect.max), P.base, true);
    let left = rect.left() + 20.0;
    let width = rect.width() - 40.0;
    let line = painter.layout(caption.to_owned(), theme::body(size::NOTE), P.muted, width);
    let title = painter.layout(name.to_owned(), theme::semibold(size::CAPTION), P.ink, width);
    let bottom = rect.bottom() - 22.0;
    let line_top = bottom - line.size().y;
    let title_top = line_top - 2.0 - title.size().y;
    painter.galley(pos2(left, title_top), title, P.ink);
    painter.galley(pos2(left, line_top), line, P.muted);
    // Fio de acento acima da legenda.
    painter.rect_filled(Rect::from_min_size(pos2(left, title_top - 10.0), vec2(36.0, 3.0)), 1.0, P.accent);
}

/// Sem arte: degrade petroleo, faixas e o nome do jogo (gerado em codigo).
fn placeholder(painter: &Painter, rect: Rect, art: &Art, name: &str) {
    let mut mesh = egui::Mesh::default();
    let top = Color32::from_rgb(0x12, 0x3c, 0x48);
    let bottom = Color32::from_rgb(0x05, 0x0d, 0x10);
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), P.base);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
    let clip = painter.with_clip_rect(rect);
    for i in 0..6 {
        let x = rect.left() - 40.0 + i as f32 * 70.0;
        clip.line_segment(
            [pos2(x, rect.bottom()), pos2(x + 260.0, rect.top())],
            egui::Stroke::new(1.0, P.accent.gamma_multiply(0.12)),
        );
    }
    if let Some(logo) = &art.logo {
        let side = rect.width() * 0.6;
        let r = Rect::from_center_size(pos2(rect.center().x, rect.top() + rect.height() * 0.32), vec2(side, side));
        painter.image(logo.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    } else {
        let g = painter.layout(name.to_owned(), theme::title(30.0), P.accent.gamma_multiply(0.9), rect.width() - 48.0);
        painter.galley(pos2(rect.left() + 24.0, rect.top() + rect.height() * 0.22), g, P.accent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_crops_the_long_side_around_the_focus() {
        // Imagem 1024x1536 numa faixa 300x640: corta na horizontal.
        let uv = cover_uv(vec2(1024.0, 1536.0), vec2(300.0, 640.0), vec2(0.5, 0.5));
        assert!((uv.height() - 1.0).abs() < 1e-5);
        assert!((uv.width() - 0.703).abs() < 0.01, "{uv:?}");
        assert!((uv.center().x - 0.5).abs() < 1e-5);
        // Imagem larga: corta na horizontal muito mais; foco na borda e preso.
        let uv = cover_uv(vec2(2188.0, 718.0), vec2(300.0, 640.0), vec2(0.0, 0.5));
        assert_eq!(uv.left(), 0.0);
        assert!(uv.width() < 0.2);
    }
}
