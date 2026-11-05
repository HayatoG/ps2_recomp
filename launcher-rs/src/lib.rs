//! Nucleo do launcher (sem UI): a CLI e, no futuro, a GUI chamam as mesmas funcoes.
pub mod assets;
pub mod config;
pub mod dump;
pub mod error;
pub mod fsx;
pub mod iso;
pub mod hw;
pub mod log;
pub mod options;
pub mod run;
pub mod settings;
#[cfg(feature = "gui")]
pub mod gui;
