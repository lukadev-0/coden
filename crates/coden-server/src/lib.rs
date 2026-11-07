mod api;
mod config;

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use axum::Router;
use axum::body::HttpBody;
use axum::extract::{FromRef, MatchedPath, Request};
use axum::middleware::{self, Next};
use axum::response::Response;
use config::Config;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, error, field, info, info_span, instrument};

use crate::api::ApiState;

#[derive(Clone)]
struct App {
    api_state: Arc<ApiState>,
}

impl FromRef<App> for Arc<ApiState> {
    fn from_ref(input: &App) -> Self {
        input.api_state.clone()
    }
}

pub async fn run(cancel: CancellationToken) -> Result<()> {
    let config = Config::from_env().context("could not load configuration")?;
    let app = init(&config)
        .await
        .context("could not initialize app state")?;

    let router = Router::new().merge(api::router());

    #[cfg(copyright_page)]
    let router = {
        let html = include_str!(concat!(env!("OUT_DIR"), "/copyright.html"));
        router.route(
            "/copyright",
            axum::routing::get(async move || axum::response::Html(html)),
        )
    };

    let router = router
        .layer(middleware::from_fn(request_tracing))
        .with_state(app.clone());

    let listener = TcpListener::bind(&config.http_addr)
        .await
        .with_context(|| format!("could not bind HTTP listener to {}", config.http_addr))?;

    info!(http_addr = %listener.local_addr()?, "listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(cancel.cancelled_owned())
        .await
        .context("error in HTTP server")?;

    app.api_state.shutdown().await?;

    info!("goodbye!");

    Ok(())
}

#[instrument]
async fn init(config: &Config) -> Result<App> {
    let api_state = Arc::new(ApiState::init(config).await?);
    Ok(App { api_state })
}

async fn request_tracing(req: Request, next: Next) -> Response {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string());
    let method = req.method().clone();

    let span = {
        let span_name = match &route {
            Some(route) => format_args!("{} {}", req.method(), *route),
            None => format_args!("{}", req.method()),
        };
        info_span!(
            "http.request",
            otel.name = span_name,
            otel.kind = "server",
            otel.status_code = field::Empty,
            url.original = %req.uri(),
            http.route = route,
            http.request.body.size = req.body().size_hint().exact(),
            http.request.method = %method,
            http.response.body.size = field::Empty,
            http.response.status_code = field::Empty,
        )
    };

    let start = Instant::now();
    let res = next.run(req).instrument(span.clone()).await;
    let duration = start.elapsed();
    span.record("http.response.body.size", res.body().size_hint().exact());
    span.record("http.response.status_code", res.status().as_u16());
    span.record(
        "otel.status_code",
        match res.status().is_server_error() {
            true => "error",
            false => "ok",
        },
    );

    if let Some(e) = res.extensions().get::<Arc<anyhow::Error>>() {
        error!(parent: &span, exception.message = ?e, "http.server.request.exception");
    }

    info!(
        parent: &span,
        histogram.http.server.request.duration = duration.as_secs_f64(),
        http.route = route,
        http.request.method = %method,
        http.response.status_code = res.status().as_u16(),
    );

    res
}
