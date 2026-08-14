#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum HttpError {
    Protocol(#[from] anyhow::Error),
}
