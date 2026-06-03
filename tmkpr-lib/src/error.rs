use thiserror::Error;

pub type TmkprResult<T> = Result<T, TmkprError>;

#[derive(Debug, Error)]
pub enum TmkprError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("{entity} not found: `{id}`")]
    NotFound { entity: &'static str, id: String },

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("a tracking session is already active (entry {id})")]
    AlreadyTracking { id: String },

    #[error("no active tracking session")]
    NotTracking,

    #[error("entry start time must be before finish time")]
    InvalidTimeRange,

    #[error("configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not parse date/time `{input}`: {reason}")]
    DateParse { input: String, reason: String },

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("obsidian logging error")]
    Obsidian,
}
