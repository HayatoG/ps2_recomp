//! Janela do launcher (`ps2launch-gui`); a logica fica em `ps2launch::gui`.
#![windows_subsystem = "windows"]

fn main() -> std::process::ExitCode {
    ps2launch::gui::main()
}
