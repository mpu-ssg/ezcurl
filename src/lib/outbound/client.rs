use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use derive_more::From;
use futures::future::BoxFuture;
use http::{Response, request::Parts};
use reqwest::{Client, Url, tls::Version};
use reqwest_cookie_store::CookieStoreRwLock;
use tower::{Service, ServiceBuilder, util::BoxCloneSyncService};

use crate::domain::client::{
    models::http::HttpError,
    ports::{HttpRequest, HttpResponse},
};

#[derive(Debug, Clone, From)]
pub struct ReqwestClient(BoxCloneSyncService<HttpRequest, HttpResponse, HttpError>);

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new(
            Client::builder()
                .tls_backend_rustls()
                .tls_version_min(Version::TLS_1_2)
                .https_only(true)
                .cookie_provider(Arc::new(CookieStoreRwLock::default()))
                .timeout(Duration::from_secs(120))
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .expect("static parameters must work"),
        )
    }
}

impl Service<HttpRequest> for ReqwestClient {
    type Response = HttpResponse;
    type Error = HttpError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.0
            .poll_ready(cx)
            .map_err(|err| HttpError::from(anyhow!(err)))
    }

    fn call(&mut self, req: HttpRequest) -> Self::Future {
        self.0.call(req)
    }
}

impl ReqwestClient {
    pub fn new(client: reqwest::Client) -> Self {
        let inner = ServiceBuilder::new()
            .map_err(|err| HttpError::from(anyhow::Error::from(err)))
            .map_request({
                let client = client.clone();
                move |req| convert_request_to_service(&client, req)
            })
            .map_future(|fut| async {
                let res = fut.await?;
                convert_response_from_service(res).await
            })
            .boxed_clone_sync()
            .service(client);

        Self(inner)
    }
}

fn convert_request_to_service(client: &reqwest::Client, req: HttpRequest) -> reqwest::Request {
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
}

async fn convert_response_from_service(
    res: reqwest::Response,
) -> Result<HttpResponse, reqwest::Error> {
    let headers = res.headers().clone();
    let status = res.status();
    let version = res.version();
    let body = res.text().await?;

    Ok(headers
        .into_iter()
        .filter_map(|(key, value)| Some((key?, value)))
        .fold(
            Response::builder().status(status).version(version),
            |res, (key, value)| res.header(key, value),
        )
        .body(body)
        .expect("response already parsed by reqwest so it must be valid"))
}
