use anyhow::anyhow;
use async_trait::async_trait;
use http::{Response, request::Parts};
use reqwest::Url;
use tower::{
    BoxError, Service, ServiceBuilder, ServiceExt,
    load::{CompleteOnResponse, PendingRequests},
    util::BoxCloneSyncService,
};

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
    pub fn new(client: reqwest::Client) -> Self {
        let service = PendingRequests::new(
            client
                .clone()
                .map_request(move |req: HttpRequest| {
                    let (
                        Parts {
                            uri,
                            method,
                            headers,
                            version,
                            ..
                        },
                        body,
                    ) = req.into_parts();

                    client
                        .request(
                            method,
                            uri.to_string().parse::<Url>().expect("already parsed"),
                        )
                        .headers(headers)
                        .version(version)
                        .body(body)
                        .build()
                        .expect("parts are supposed valid from their type")
                })
                .map_future(|res| async {
                    let res = res.await?;
                    let headers = res.headers().clone();
                    let status = res.status();
                    let version = res.version();
                    let body = res.text().await?;

                    Ok::<_, reqwest::Error>(
                        headers
                            .into_iter()
                            .filter_map(|(key, value)| Some((key?, value)))
                            .fold(
                                Response::builder().status(status).version(version),
                                |res, (key, value)| res.header(key, value),
                            )
                            .body(body)
                            .expect("response already parsed by reqwest so it must be valid"),
                    )
                }),
            CompleteOnResponse::default(),
        );
        let service = ServiceBuilder::new()
            .map_err(|err: BoxError| HttpError::from(anyhow!(err)))
            .buffer(0x2000)
            .boxed_clone_sync()
            .service(service);
        Self(service)
    }
}
