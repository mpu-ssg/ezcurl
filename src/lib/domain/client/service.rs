use async_trait::async_trait;
use tower::{Service, ServiceBuilder, util::BoxCloneSyncService};

use crate::domain::client::{
    models::http::HttpError,
    ports::{ClientService, HttpRequest, HttpResponse},
};

#[derive(Debug, Clone)]
pub struct HttpClient(BoxCloneSyncService<HttpRequest, HttpResponse, HttpError>);

#[async_trait]
impl ClientService for HttpClient {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let mut service = self.0.clone();
        service.call(request).await
    }
}

impl HttpClient {
    pub fn new<C>(client: C) -> Self
    where
        C: Service<HttpRequest, Response = HttpResponse, Error = HttpError>
            + Clone
            + Send
            + Sync
            + 'static,
        C::Future: Send + Sync + 'static,
    {
        let service = ServiceBuilder::new().boxed_clone_sync().service(client);
        Self(service)
    }
}
