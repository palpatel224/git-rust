use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hex encoding/decoding error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("String conversion error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[error("Formatting error: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("Network request error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Invalid Git object format: {message}")]
    InvalidObjectFormat { message: String },

    #[error("{message}")]
    Generic { message: String },
}

impl GitError {
    pub fn any(message: impl Into<String>) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    pub fn invalid_object_format(message: impl Into<String>) -> Self {
        Self::InvalidObjectFormat {
            message: message.into(),
        }
    }
}
