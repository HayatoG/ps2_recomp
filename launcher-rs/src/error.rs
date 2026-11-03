//! Tipo de erro unico do launcher (mensagens pensadas para o usuario final).
use std::fmt;

#[derive(Debug)]
pub struct Error(pub String);

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error(format!("erro de E/S: {e}"))
    }
}

/// Atalho: `err!("texto {}", x)`.
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => { $crate::error::Error(format!($($arg)*)) };
}
