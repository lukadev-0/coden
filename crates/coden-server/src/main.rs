use anyhow::Result;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tokio_util::sync::CancellationToken;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let cancel = CancellationToken::new();
    let mut task = tokio::spawn(coden_server::run(cancel.clone()));

    tokio::select! {
        res = &mut task => {
            res??;
        }
        _ = tokio::signal::ctrl_c() => {
            cancel.cancel();
            task.await??;
        }
    }

    Ok(())
}

fn init_tracing() -> Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer();

    let otel_enabled = std::env::var_os("CODEN_OTEL").is_some_and(|s| !s.is_empty());
    let otel_layer = otel_enabled.then(otel_layer).transpose()?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    Ok(())
}

fn otel_layer<S>() -> Result<impl Layer<S>>
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span>,
{
    let otlp_span_exporter = opentelemetry_otlp::SpanExporter::builder().build()?;
    let otlp_metric_exporter = opentelemetry_otlp::MetricExporter::builder().build()?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(otlp_span_exporter)
        .build();
    let tracer = tracer_provider.tracer(env!("CARGO_PKG_NAME"));

    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(otlp_metric_exporter)
        .build();

    let otel_tracing_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let otel_metrics_layer = tracing_opentelemetry::MetricsLayer::new(meter_provider);

    Ok(otel_tracing_layer.and_then(otel_metrics_layer))
}
