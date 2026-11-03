//! Controles desenhados a mao (cartoes, chips, abas, botao JOGAR inclinado) com foco visivel.
use super::theme::{self, size, P};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Mesh, Painter, Pos2, Rect, Response, RichText, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2,
};

/// Contorno ciano de foco (teclado/controle).
pub fn focus_ring(painter: &Painter, rect: Rect) {
    painter.rect_stroke(rect.expand(3.0), CornerRadius::same(9), Stroke::new(2.0, P.accent), StrokeKind::Outside);
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(theme::semibold(size::HEADING)).color(P.muted));
}

pub fn note(ui: &mut Ui, text: &str, color: Color32) -> Response {
    ui.add(egui::Label::new(RichText::new(text).font(theme::body(size::NOTE)).color(color)).wrap())
}

/// Botao com contorno ciano.
pub fn outline_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), theme::semibold(size::BUTTON), P.ink);
    let size = vec2((galley.size().x + 26.0).max(96.0), 34.0);
    let (rect, response) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let hovered = enabled && response.hovered();
    let (stroke, ink) = if enabled { (P.accent, if hovered { P.ink } else { P.accent }) } else { (P.disabled, P.disabled) };
    let fill = if hovered { P.panel_hi } else { Color32::TRANSPARENT };
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(6), fill, Stroke::new(1.5, stroke), StrokeKind::Inside);
    p.text(rect.center(), Align2::CENTER_CENTER, text, theme::semibold(size::BUTTON), ink);
    if response.has_focus() {
        focus_ring(p, rect);
    }
    if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// Botao discreto (texto sublinhado ao passar o mouse), para o rodape.
pub fn link_button(ui: &mut Ui, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), theme::body(size::NOTE), P.muted);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(4.0, 4.0), Sense::click());
    let color = if response.hovered() || response.has_focus() { P.accent } else { P.muted };
    let p = ui.painter();
    p.text(rect.left_center() + vec2(2.0, 0.0), Align2::LEFT_CENTER, text, theme::body(size::NOTE), color);
    if response.hovered() {
        p.hline(rect.x_range().shrink(2.0), rect.bottom() - 1.0, Stroke::new(1.0, P.accent));
    }
    if response.has_focus() {
        p.rect_stroke(rect.expand(2.0), CornerRadius::same(4), Stroke::new(2.0, P.accent), StrokeKind::Outside);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// Um cartao de perfil.
pub struct Card<'a> {
    pub title: &'a str,
    pub note: &'a str,
    pub badge: Option<&'a str>,
    pub selected: bool,
}

pub fn card(ui: &mut Ui, size: Vec2, c: &Card<'_>) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let focused = response.has_focus();
    let fill = if c.selected {
        P.panel_hi
    } else if response.hovered() || focused {
        P.panel
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if c.selected { Stroke::new(2.0, P.accent) } else { Stroke::new(1.0, P.line) };
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);
    if focused {
        focus_ring(p, rect);
    }
    let pad = 12.0;
    // Titulo: encolhe ate caber na largura do cartao.
    let ink = if c.selected { P.accent } else { P.ink };
    let mut ts = size::CARD_TITLE;
    let mut tg = p.layout_no_wrap(c.title.to_owned(), theme::bold(ts), ink);
    while tg.size().x > size.x - 2.0 * pad && ts > size::NOTE {
        ts -= 0.5;
        tg = p.layout_no_wrap(c.title.to_owned(), theme::bold(ts), ink);
    }
    p.galley(rect.left_top() + vec2(pad, pad), tg, ink);
    if let Some(badge) = c.badge {
        let g = p.layout_no_wrap(badge.to_owned(), theme::semibold(size::BADGE - 1.0), P.on_accent);
        let tab = Rect::from_min_size(pos2(rect.center().x - g.size().x / 2.0 - 8.0, rect.top() - 11.0), g.size() + vec2(16.0, 2.0));
        let fill = if c.selected { P.accent } else { P.muted };
        p.rect_filled(tab, CornerRadius::same(5), fill);
        p.galley(tab.min + vec2(8.0, 1.0), g, P.on_accent);
    }
    let g = p.layout(c.note.to_owned(), theme::body(size::NOTE), P.muted, size.x - 2.0 * pad);
    p.galley(rect.left_top() + vec2(pad, pad + 28.0), g, P.muted);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// Fileira de escolhas. `disabled` = motivo (mostrado ao passar o mouse) e o chip fica cinza.
pub struct Chip<'a, T> {
    pub value: T,
    pub label: &'a str,
    pub badge: Option<&'a str>,
    pub disabled: Option<&'a str>,
}

/// Desenha os chips; devolve o valor clicado (se mudou). Setas esquerda/direita andam entre
/// chips com foco (comportamento padrao do egui).
pub fn chips<T: PartialEq + Copy>(ui: &mut Ui, options: &[Chip<'_, T>], current: T) -> Option<T> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for o in options {
            let enabled = o.disabled.is_none();
            let on = o.value == current;
            let text_w = ui.painter().layout_no_wrap(o.label.to_owned(), theme::semibold(size::BUTTON), P.ink).size().x;
            // A cor da galley vence a do painter: ja montar com a cor final.
            let badge_color = if on { P.on_accent } else if enabled { P.warm } else { P.disabled };
            let badge_g = o.badge.map(|b| ui.painter().layout_no_wrap(b.to_owned(), theme::body(size::NOTE), badge_color));
            let badge_w = badge_g.as_ref().map(|g| g.size().x + 14.0).unwrap_or(0.0);
            let size = vec2(text_w + badge_w + 28.0, 34.0);
            let (rect, response) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
            let p = ui.painter();
            let (fill, stroke, ink) = match (enabled, on) {
                (false, _) => (Color32::TRANSPARENT, Stroke::new(1.0, P.line), P.disabled),
                (true, true) => (P.accent, Stroke::NONE, P.on_accent),
                (true, false) if response.hovered() => (P.panel_hi, Stroke::new(1.0, P.muted), P.ink),
                (true, false) => (P.panel, Stroke::new(1.0, P.line), P.ink),
            };
            p.rect(rect, CornerRadius::same(17), fill, stroke, StrokeKind::Inside);
            p.text(rect.left_center() + vec2(14.0, 0.0), Align2::LEFT_CENTER, o.label, theme::semibold(size::BUTTON), ink);
            if let Some(g) = badge_g {
                let at = pos2(rect.left() + 14.0 + text_w + 10.0, rect.center().y - g.size().y / 2.0);
                p.galley(at, g, badge_color);
            }
            if response.has_focus() {
                p.rect_stroke(rect.expand(3.0), CornerRadius::same(19), Stroke::new(2.0, P.accent), StrokeKind::Outside);
            }
            let response = match o.disabled {
                Some(why) => response.on_hover_text(why),
                None => response.on_hover_cursor(CursorIcon::PointingHand),
            };
            if enabled && response.clicked() && !on {
                picked = Some(o.value);
            }
        }
    });
    picked
}

/// Interruptor com texto; `disabled` = motivo.
pub fn toggle(ui: &mut Ui, on: &mut bool, text: &str, disabled: Option<&str>) -> Response {
    let enabled = disabled.is_none();
    let g = ui.painter().layout_no_wrap(text.to_owned(), theme::body(size::BODY), P.ink);
    let size = vec2(44.0 + 10.0 + g.size().x + 6.0, 30.0);
    let (rect, mut response) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    if enabled && response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let p = ui.painter();
    let track = Rect::from_min_size(pos2(rect.left(), rect.center().y - 11.0), vec2(44.0, 22.0));
    let (fill, knob) = match (enabled, *on) {
        (false, _) => (P.panel, P.disabled),
        (true, true) => (P.accent, P.on_accent),
        (true, false) => (P.panel_hi, P.muted),
    };
    p.rect(track, CornerRadius::same(11), fill, Stroke::new(1.0, P.line), StrokeKind::Inside);
    let x = if *on { track.right() - 11.0 } else { track.left() + 11.0 };
    p.circle_filled(pos2(x, track.center().y), 8.0, knob);
    p.text(
        pos2(track.right() + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme::body(size::BODY),
        if enabled { P.ink } else { P.disabled },
    );
    if response.has_focus() {
        p.rect_stroke(rect.expand(3.0), CornerRadius::same(8), Stroke::new(2.0, P.accent), StrokeKind::Outside);
    }
    match disabled {
        Some(why) => response.on_hover_text(why),
        None => response.on_hover_cursor(CursorIcon::PointingHand),
    }
}

/// Abas com sublinhado ciano e linha divisoria; devolve a aba clicada.
pub fn tabs_bar(ui: &mut Ui, names: &[&str], current: usize) -> Option<usize> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 28.0;
        for (i, name) in names.iter().enumerate() {
            let on = i == current;
            let size = ui.painter().layout_no_wrap((*name).to_owned(), theme::semibold(size::TAB), P.muted).size();
            // So clique (sem parada de foco): pelo teclado/controle as abas trocam com
            // Ctrl+Tab, Q/E ou LB/RB, e o Tab vai direto ao conteudo.
            let (rect, response) = ui.allocate_exact_size(size + vec2(0.0, 10.0), Sense::CLICK);
            let color = if on || response.hovered() { P.ink } else { P.muted };
            ui.painter().text(rect.left_top(), Align2::LEFT_TOP, *name, theme::semibold(size::TAB), color);
            if on {
                ui.painter().rect_filled(
                    Rect::from_min_size(pos2(rect.left(), rect.bottom() - 3.0), vec2(rect.width(), 3.0)),
                    CornerRadius::same(1),
                    P.accent,
                );
            }
            if response.on_hover_cursor(CursorIcon::PointingHand).clicked() && !on {
                picked = Some(i);
            }
        }
    });
    let y = ui.cursor().top() - 4.0;
    let r = ui.max_rect();
    ui.painter().hline(r.left()..=r.right(), y, Stroke::new(1.0, P.line));
    picked
}

/// Barra de progresso fina.
pub fn progress_bar(ui: &mut Ui, fraction: f32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(4), P.line);
    let done = Rect::from_min_size(rect.min, vec2(w * fraction.clamp(0.0, 1.0), rect.height()));
    ui.painter().rect_filled(done, CornerRadius::same(4), P.accent);
}

/// O botao JOGAR: paralelogramo ciano com texto escuro; cinza quando nao esta pronto.
pub fn play_plate(ui: &mut Ui, rect: Rect, label: &str, ready: bool) -> Response {
    let response = ui.interact(rect, ui.scope_id().with("ps2l-play"), Sense::click());
    let fill = match (ready, response.hovered()) {
        (false, _) => P.panel_hi,
        (true, true) => P.accent_hover,
        (true, false) => P.accent,
    };
    let slant = 16.0;
    let pts = |r: Rect| -> Vec<Pos2> {
        vec![pos2(r.left() + slant, r.top()), r.right_top(), pos2(r.right() - slant, r.bottom()), r.left_bottom()]
    };
    let p = ui.painter();
    // Sombra quente discreta deslocada (o secundario laranja).
    if ready {
        p.add(Shape::convex_polygon(pts(rect.translate(vec2(5.0, 5.0))), P.warm.gamma_multiply(0.55), Stroke::NONE));
    }
    p.add(Shape::convex_polygon(pts(rect), fill, Stroke::NONE));
    p.text(rect.center(), Align2::CENTER_CENTER, label, theme::title(size::PLAY), if ready { P.on_accent } else { P.disabled });
    if response.has_focus() {
        p.add(Shape::closed_line(pts(rect.expand(4.0)), Stroke::new(2.0, P.ink)));
    }
    if ready {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

/// Degrade de transparente para `color` (para a direita ou para baixo).
pub fn gradient(painter: &Painter, rect: Rect, color: Color32, down: bool) {
    let clear = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    let (a, b, c, d) = if down { (clear, clear, color, color) } else { (clear, color, color, clear) };
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), a);
    mesh.colored_vertex(rect.right_top(), b);
    mesh.colored_vertex(rect.right_bottom(), c);
    mesh.colored_vertex(rect.left_bottom(), d);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// Faixas inclinadas no canto superior direito (decoracao).
pub fn stripes(painter: &Painter, full: Rect) {
    let painter = painter.with_clip_rect(full);
    let (slant, depth) = (52.0, 96.0);
    for (i, (w, color)) in [(22.0, P.accent.gamma_multiply(0.85)), (10.0, P.warm.gamma_multiply(0.7))].into_iter().enumerate() {
        let x = full.right() - 86.0 + i as f32 * 36.0;
        painter.add(Shape::convex_polygon(
            vec![pos2(x, full.top()), pos2(x + w, full.top()), pos2(x + w - slant, full.top() + depth), pos2(x - slant, full.top() + depth)],
            color,
            Stroke::NONE,
        ));
    }
}
