use ezcurl::domain::client::models::http::HttpError;

use crate::history::HistoryError;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum EzcurlError {
    InvalidUrl(#[from] http::uri::InvalidUri),
    #[error("invalid header line: {0}")]
    InvalidHeader(String),
    Network(#[from] HttpError),
    History(#[from] HistoryError),
    Terminal(#[from] std::io::Error),
}
