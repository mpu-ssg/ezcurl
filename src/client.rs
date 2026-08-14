use std::{sync::Arc, time::Duration};

use reqwest::tls;
use reqwest_cookie_store::CookieStoreRwLock;

use crate::{error::EzcurlError, request::HttpRequest, response::HttpResponse};

pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .tls_backend_rustls()
                .tls_version_min(tls::Version::TLS_1_2)
                .https_only(false)
                .cookie_provider(Arc::new(CookieStoreRwLock::default()))
                .timeout(Duration::from_secs(120))
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .expect("static parameters must work"),
        }
    }

    pub async fn send(&self, http_request: &HttpRequest) -> Result<HttpResponse, EzcurlError> {
        let url = reqwest::Url::parse(http_request.url())
            .map_err(|_| EzcurlError::InvalidUrl(http_request.url().to_string()))?;

        let mut builder = self
            .client
            .request(http_request.method().as_reqwest_method(), url);

        for (name, value) in http_request.header_values()? {
            builder = builder.header(name, value);
        }

        if let Some(body) = http_request.body() {
            builder = builder.body(body.to_vec());
        }

        let response = builder.send().await?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.to_str().unwrap_or("<invalid header>").to_string(),
                )
            })
            .collect();

        let body = response.bytes().await?.to_vec();
        Ok(HttpResponse::new(status, headers, body))
    }
}
