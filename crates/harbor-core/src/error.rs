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

    pub(crate) fn from_constraint(err: sqlx::Error, unique_message: &str) -> Self {
        if err
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
        {
            Self::Message(unique_message.into())
        } else if err
            .as_database_error()
            .is_some_and(|error| error.is_foreign_key_violation())
        {
            Self::Message("referenced row not found".into())
        } else {
            Self::from(err)
        }
    }
}
