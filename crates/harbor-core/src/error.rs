use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unimplemented: {0}")]
    Unimplemented(&'static str),
    #[error("{0}")]
    Message(String),
}

impl Error {
    pub fn unimplemented(name: &'static str) -> Self {
        Self::Unimplemented(name)
    }
}
