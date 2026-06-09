use std::fmt::Display;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0:?}")]
    Anyhow(#[from] anyhow::Error),
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        anyhow::anyhow!("{msg}").into()
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        anyhow::anyhow!("{msg}").into()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
