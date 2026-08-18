use std::task::Poll;

use futures::{FutureExt, future::BoxFuture};
use tower::Service;

use crate::inbound::cli::CliError;

pub mod events;

pub struct EventHandler;

impl Service<events::CliEvent> for EventHandler {
    type Response = ();
    type Error = CliError;
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: events::CliEvent) -> Self::Future {
        async { Ok(()) }.boxed()
    }
}
