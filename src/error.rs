use std::fmt;

#[derive(Debug)]
pub enum EzcurlError {
    InvalidUrl(String),
    InvalidHeader(String),
    Network(reqwest::Error),
    History(crate::history::HistoryError),
    Terminal(std::io::Error),
}

impl From<reqwest::Error> for EzcurlError {
    fn from(error: reqwest::Error) -> Self {
        Self::Network(error)
    }
}

impl From<std::io::Error> for EzcurlError {
    fn from(error: std::io::Error) -> Self {
        Self::Terminal(error)
    }
}

impl From<crate::history::HistoryError> for EzcurlError {
    fn from(error: crate::history::HistoryError) -> Self {
        Self::History(error)
    }
}

impl fmt::Display for EzcurlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EzcurlError::InvalidUrl(url) => {
                if url.is_empty() {
                    f.write_str("No URL provided")
                } else {
                    write!(f, "Invalid URL: {url}")
                }
            }
            EzcurlError::InvalidHeader(header) => write!(f, "invalid header line: {header}"),
            EzcurlError::Network(error) => write!(f, "network error: {error}"),
            EzcurlError::History(error) => write!(f, "history error: {error}"),
            EzcurlError::Terminal(error) => write!(f, "Terminal error: {error}"),
        }
    }
}
