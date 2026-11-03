//! Controle (gilrs) na janela: o D-pad vira Tab/Shift+Tab/setas e o A vira Espaco, para o foco do
//! proprio egui fazer o trabalho (e o contorno de foco aparecer). B, LB/RB e Start voltam como
//! acoes do launcher.
use eframe::egui::{self, Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Down,
    Up,
    Left,
    Right,
    Press,
    Back,
    NextTab,
    PrevTab,
    Play,
}

pub struct Pad {
    gilrs: Option<gilrs::Gilrs>,
}

impl Pad {
    pub fn new() -> Pad {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                crate::warn!("controle indisponivel (gilrs): {e}");
                None
            }
        };
        Pad { gilrs }
    }

    /// Botoes apertados desde a ultima chamada (uma acao por aperto, nunca por quadro).
    pub fn poll(&mut self) -> Vec<Nav> {
        let mut out = Vec::new();
        let Some(g) = self.gilrs.as_mut() else { return out };
        while let Some(ev) = g.next_event() {
            if let gilrs::EventType::ButtonPressed(b, _) = ev.event {
                use gilrs::Button as B;
                let nav = match b {
                    B::DPadDown => Nav::Down,
                    B::DPadUp => Nav::Up,
                    B::DPadLeft => Nav::Left,
                    B::DPadRight => Nav::Right,
                    B::South => Nav::Press,
                    B::East => Nav::Back,
                    B::RightTrigger => Nav::NextTab,
                    B::LeftTrigger => Nav::PrevTab,
                    B::Start => Nav::Play,
                    _ => continue,
                };
                out.push(nav);
            }
        }
        out
    }

    /// Nome do primeiro controle conectado.
    pub fn connected(&self) -> Option<String> {
        let g = self.gilrs.as_ref()?;
        g.gamepads().find(|(_, p)| p.is_connected()).map(|(_, p)| p.name().to_string())
    }
}

fn key(raw: &mut egui::RawInput, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        raw.events.push(egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
    }
}

/// Converte os passos de foco em teclas. `typing` (campo de texto com foco) impede o A de
/// digitar espaco; `nothing_focused` faz setas/A focarem o primeiro controle.
pub fn inject(raw: &mut egui::RawInput, navs: &[Nav], typing: bool, nothing_focused: bool) {
    for nav in navs {
        match nav {
            Nav::Down => key(raw, Key::Tab, Modifiers::NONE),
            Nav::Up => key(raw, Key::Tab, Modifiers::SHIFT),
            Nav::Left | Nav::Right | Nav::Press if nothing_focused => key(raw, Key::Tab, Modifiers::NONE),
            Nav::Left => key(raw, Key::ArrowLeft, Modifiers::NONE),
            Nav::Right => key(raw, Key::ArrowRight, Modifiers::NONE),
            Nav::Press if !typing => key(raw, Key::Space, Modifiers::NONE),
            _ => {}
        }
    }
}
