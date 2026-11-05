//! Textos da janela. Um idioma = uma instancia de [`Strings`]; hoje so pt-BR ([`PT_BR`]).
//! Para adicionar ingles: criar `EN` com os mesmos campos e escolher em [`get`].
//! Marcadores `{}` sao preenchidos em ordem por [`fill`].

pub struct Strings {
    pub app_suffix: &'static str,
    // Status (linha abaixo do titulo)
    pub status_ready: &'static str,
    pub status_checking: &'static str,
    pub status_need_dump: &'static str,
    pub status_bad_dump: &'static str,
    pub status_need_extract: &'static str,
    pub status_extracting: &'static str,
    pub status_launching: &'static str,
    pub status_no_port: &'static str,
    // Abas
    pub tab_play: &'static str,
    pub tab_picture: &'static str,
    pub tab_controls: &'static str,
    // Aba Jogar
    pub disc_heading: &'static str,
    pub disc_hint: &'static str,
    pub browse: &'static str,
    pub browse_title: &'static str,
    pub disc_filter: &'static str,
    pub found: &'static str,
    pub found_auto: &'static str,
    pub checking_disc: &'static str,
    pub profile_heading: &'static str,
    pub badge_default: &'static str,
    pub profile_original: &'static str,
    pub profile_original_note: &'static str,
    pub profile_recommended: &'static str,
    pub profile_recommended_note: &'static str,
    pub profile_deck: &'static str,
    pub profile_deck_note: &'static str,
    pub profile_custom: &'static str,
    pub profile_custom_note: &'static str,
    pub deck_fps_label: &'static str,
    pub data_heading: &'static str,
    pub data_status: &'static str,
    pub data_free: &'static str,
    pub data_complete: &'static str,
    pub data_none: &'static str,
    pub extract: &'static str,
    pub extracting_file: &'static str,
    pub extract_done: &'static str,
    pub extract_failed: &'static str,
    // Aba Imagem
    pub renderer_heading: &'static str,
    pub renderer_cpu: &'static str,
    pub renderer_d3d11: &'static str,
    pub renderer_vulkan: &'static str,
    pub renderer_d3d12: &'static str,
    pub renderer_cpu_thread: &'static str,
    pub renderer_hw: &'static str,
    pub badge_experimental: &'static str,
    pub badge_soon: &'static str,
    pub soon_reason: &'static str,
    pub d3d11_warning: &'static str,
    pub fps_heading: &'static str,
    pub fps_unlimited: &'static str,
    pub vsync_label: &'static str,
    pub vsync_cpu_reason: &'static str,
    pub heartbeat_label: &'static str,
    pub heartbeat_note: &'static str,
    pub more_heading: &'static str,
    pub internal_res: &'static str,
    pub fullscreen: &'static str,
    pub picture_profile_note: &'static str,
    // Aba Controles
    pub controls_col_ps2: &'static str,
    pub controls_col_keyboard: &'static str,
    pub controls_col_pad: &'static str,
    pub controls_rows: &'static [[&'static str; 3]],
    pub controls_note_sticks: &'static str,
    pub controls_note_pad_wins: &'static str,
    pub pad_detected: &'static str,
    pub pad_none: &'static str,
    pub controls_launcher_nav: &'static str,
    // Rodape
    pub play: &'static str,
    pub hint_ready: &'static str,
    pub hint_extract_first: &'static str,
    pub hint_d3d11: &'static str,
    pub export_logs: &'static str,
    pub export_title: &'static str,
    pub exported: &'static str,
    pub no_logs: &'static str,
    pub open_folder: &'static str,
    pub play_disabled_checking: &'static str,
    pub play_disabled_dump: &'static str,
    pub play_disabled_busy: &'static str,
    // Erros
    pub save_failed: &'static str,
    pub launch_failed: &'static str,
    // Arte
    pub caption_line: &'static str,
}

pub static PT_BR: Strings = Strings {
    app_suffix: "ps2launch",
    status_ready: "Pronto. Pressione Jogar ou Enter.",
    status_checking: "Verificando o disco…",
    status_need_dump: "Escolha a imagem do seu disco para continuar.",
    status_bad_dump: "O disco não foi aceito. Veja o motivo abaixo.",
    status_need_extract: "Disco verificado. Jogar extrai antes os dados que faltam ({} GB).",
    status_extracting: "Extraindo os dados do disco… {}%",
    status_launching: "Iniciando o jogo…",
    status_no_port: "O port ainda não foi construído: {}",
    tab_play: "Jogar",
    tab_picture: "Imagem",
    tab_controls: "Controles",
    disc_heading: "Imagem do disco",
    disc_hint: "ISO ou BIN do seu disco (vazio = procurar na pasta do jogo)",
    browse: "Procurar…",
    browse_title: "Escolha a imagem do disco",
    disc_filter: "Imagem de disco",
    found: "Encontrado: {} ({}), edição conhecida.",
    found_auto: "Detectado na pasta do jogo: {}",
    checking_disc: "Conferindo serial, hash e tamanhos…",
    profile_heading: "Perfil",
    badge_default: "Padrão",
    profile_original: "Original",
    profile_original_note: "PAL 50 Hz, 640×448; 60 fps na tela.",
    profile_recommended: "Recomendado",
    profile_recommended_note: "Padrão seguro: CPU, 60 fps.",
    profile_deck: "Steam Deck",
    profile_deck_note: "1280×800; limite de 30, 40 ou 60.",
    profile_custom: "Personalizado",
    profile_custom_note: "Ajuste tudo na aba Imagem.",
    deck_fps_label: "Limite no Deck:",
    data_heading: "Dados extraídos",
    data_status: "{} de {} arquivos · {} de {} GB",
    data_free: "{} GB livres",
    data_complete: "Completo",
    data_none: "Sem tabela de arquivos no game.toml; os dados são extraídos ao jogar.",
    extract: "Extrair",
    extracting_file: "Extraindo {} — {} de {} MB",
    extract_done: "Extração concluída.",
    extract_failed: "A extração falhou: {}",
    renderer_heading: "Renderizador",
    renderer_cpu: "CPU (software)",
    renderer_d3d11: "Direct3D 11",
    renderer_vulkan: "Vulkan",
    renderer_d3d12: "Direct3D 12",
    renderer_cpu_thread: "CPU (thread)",
    renderer_hw: "Hardware",
    badge_experimental: "experimental",
    badge_soon: "em breve",
    soon_reason: "Ainda não existe no runtime.",
    d3d11_warning: "Experimental: abre uma SEGUNDA janela com a mesma imagem (a do jogo continua aberta, é ela que recebe teclado e controle). O Direct3D 11 só apresenta a imagem; o desenho ainda é feito na CPU.",
    fps_heading: "Limite de FPS",
    fps_unlimited: "Ilimitado",
    vsync_label: "VSync (Direct3D 11)",
    vsync_cpu_reason: "Só tem efeito com o Direct3D 11.",
    heartbeat_label: "Modo de diagnóstico",
    heartbeat_note: "Uma linha por segundo com o estado do jogo no log (logs\\run_*.log).",
    more_heading: "Mais opções",
    internal_res: "Resolução interna",
    fullscreen: "Tela cheia",
    picture_profile_note: "Mudar uma opção aqui troca o perfil para Personalizado.",
    controls_col_ps2: "PS2",
    controls_col_keyboard: "Teclado",
    controls_col_pad: "Controle Xbox",
    controls_rows: &[
        ["Cruz ✕", "X ou Espaço", "A"],
        ["Círculo ○", "C ou Esc", "B"],
        ["Quadrado □", "Z ou Num0", "X"],
        ["Triângulo △", "V ou Num1", "Y"],
        ["Direcional", "Setas ou WASD", "D-pad"],
        ["L1 / R1", "Q / E", "LB / RB"],
        ["L2 / R2", "Shift esq. / Shift dir.", "LT / RT"],
        ["Start", "Enter", "Menu"],
        ["Select", "Tab", "View"],
        ["Analógicos", "—", "Sticks esquerdo e direito"],
    ],
    controls_note_sticks: "O teclado não move os analógicos.",
    controls_note_pad_wins: "Com um controle conectado, o jogo ignora o teclado.",
    pad_detected: "Controle detectado: {}",
    pad_none: "Nenhum controle detectado agora.",
    controls_launcher_nav: "Nesta janela: D-pad move, A ativa, B volta, LB/RB trocam de aba, Start joga. No teclado: Tab/setas, Espaço/Enter, Ctrl+Tab ou Q/E, Esc fecha.",
    play: "JOGAR",
    hint_ready: "Enter ou Start também jogam. A janela fecha quando o jogo abre.",
    hint_extract_first: "Jogar extrai os dados que faltam e depois abre o jogo.",
    hint_d3d11: "Direct3D 11 experimental: vão abrir duas janelas do jogo.",
    export_logs: "Exportar logs",
    export_title: "Escolha onde salvar os logs",
    exported: "Logs copiados para {}",
    no_logs: "Ainda não há logs para exportar.",
    open_folder: "Abrir pasta do jogo",
    play_disabled_checking: "Aguarde: o disco ainda está sendo verificado.",
    play_disabled_dump: "Jogar fica disponível quando o disco original for verificado.",
    play_disabled_busy: "Aguarde a tarefa em andamento terminar.",
    save_failed: "Não consegui salvar as configurações: {}",
    launch_failed: "O jogo não iniciou: {}",
    caption_line: "{} · {}",
};

/// Idioma ativo (hoje sempre pt-BR).
pub fn get() -> &'static Strings {
    &PT_BR
}

/// Troca cada `{}` de `template` pelo proximo de `args`.
pub fn fill(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut it = args.iter();
    let mut rest = template;
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        match it.next() {
            Some(a) => out.push_str(&a.to_string()),
            None => out.push_str("{}"),
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// Regiao do `game.toml` para exibicao.
pub fn region_name(region: &str) -> &'static str {
    match region.to_ascii_uppercase().as_str() {
        "PAL" => "Europa/PAL",
        "NTSC-U" | "NTSC_U" | "USA" => "América do Norte/NTSC-U",
        "NTSC-J" | "NTSC_J" | "JAPAN" => "Japão/NTSC-J",
        _ => "região desconhecida",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_replaces_in_order() {
        assert_eq!(fill("{} de {}", &[&1, &"dois"]), "1 de dois");
        assert_eq!(fill("sem {}", &[]), "sem {}");
    }
}
