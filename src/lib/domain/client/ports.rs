use async_trait::async_trait;
use http::{Request, Response};

use crate::domain::client::models::http::HttpError;

pub type HttpRequest = Request<String>;
pub type HttpResponse = Response<String>;

#[async_trait]
pub trait ClientService: Clone + Send + Sync + 'static {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, HttpError>;
}
