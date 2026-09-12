// tecton-cli/src/commands/server.rs
use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use clap::Args;
//use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{KeyValue, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use std::time::Duration;
//use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use std::net::SocketAddr;

use tecton_core::config::CoreConfig;
use tecton_web::{AppState, create_router, create_service_router};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
//use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Args, PartialEq, Eq, Clone)]
pub struct ServerArgs {
    /// Server host
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Server port
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// Enable OpenTelemetry tracing
    #[arg(long)]
    pub otel: bool,

    /// OTLP endpoint
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otel_endpoint: Option<String>,
}

pub async fn execute(config: CoreConfig, _args: ServerArgs) -> Result<()> {
    // Initialize tracing

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,txt_block_index=debug,tower_http=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let serv_config = &config.server;
    // Initialize OpenTelemetry if requested
    if serv_config.otel && serv_config.otel_endpoint.is_some() {
        let _otel = init_opentelemetry(serv_config.otel_endpoint.as_deref());
    }

    // init_metrics();
    // Create application state
    let state = AppState::new(config.clone()).await?;

    // Build router
    let app = create_router(state.clone()).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");

                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %request_id,
                    )
                }),
            )
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(CorsLayer::permissive())
            .layer(CompressionLayer::new()),
    );
    //let app = open_api.merge(actuate_metrics);
    //.with_state(state);
    let merged_app = app.merge(create_service_router(state));

    // Start server
    //let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let addr: SocketAddr = serv_config.from.parse()?;

    tracing::info!("Starting web server on {}", addr);

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let _shutdown_future = shutdown_signal(handle.clone());

    if serv_config.enable_tls {
        // configure certificate and private key used by https
        let config = RustlsConfig::from_pem_file(
            &serv_config.cert_path.clone().unwrap(),
            &serv_config.key_path.clone().unwrap(),
        )
        .await
        .unwrap();
        axum_server::bind_rustls(addr, config)
            .handle(handle)
            .serve(merged_app.into_make_service())
            .await?;
    } else {
        axum_server::bind(addr)
            .handle(handle)
            .serve(merged_app.into_make_service())
            .await?;
    }
    Ok(())
}

/*fn actuate_metrics() -> Router {
    let recorder_handle = setup_metrics_recorder();
    Router::new().route("/metrics", get(move || ready(recorder_handle.render())))
}*/

fn init_opentelemetry(endpoint: Option<&str>) -> Result<SdkTracerProvider> {
    /* let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .unwrap();

    let provider = SdkTracerProvider::builder()
        // Customize sampling strategy
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        // If export trace to AWS X-Ray, you can use XrayIdGenerator
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build();

    Ok(provider)*/
    let endpoint = endpoint.unwrap_or("http://localhost:4317");
    /*let exporter = opentelemetry_otlp::MetricExporter::builder()
    .with_tonic()
    //.with_protocol(Protocol::HttpBinary)
    .with_endpoint(endpoint)
    .build()?;
    */
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            opentelemetry_sdk::Resource::builder_empty()
                .with_service_name("tecton")
                .with_attributes(vec![KeyValue::new(
                    "service.version",
                    env!("CARGO_PKG_VERSION"),
                )])
                .build(),
        )
        .build();

    let tracer = provider.tracer("tecton");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer())
        .init();
    /*let meter_provider = SdkMeterProvider::builder()
    .with_periodic_exporter(exporter)
    .with_resource(
        opentelemetry_sdk::Resource::builder()
            .with_service_name("tecton")
            .with_attributes(vec![KeyValue::new(
                "service.version",
                env!("CARGO_PKG_VERSION"),
            )])
            .build(),
    )
    .build();*/

    //global::set_meter_provider(meter_provider.clone());

    Ok(provider)
}

async fn shutdown_signal(handle: axum_server::Handle<SocketAddr>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
    handle.graceful_shutdown(Some(Duration::from_secs(10))); // 10 secs is how long docker will wait
    // to force shutdown
}
