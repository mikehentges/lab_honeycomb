use actix_web::{web, App, HttpServer};
use opentelemetry::sdk::trace as sdktrace;
use opentelemetry::trace::TraceError;

use tonic::{
    metadata::{MetadataKey, MetadataMap},
    transport::ClientTlsConfig,
};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::Registry;
use url::Url;

use opentelemetry_otlp::WithExportConfig;
use std::str::FromStr;
use std::{env::var, error::Error};
use tracing_subscriber::layer::SubscriberExt;

const ENDPOINT: &str = "OTLP_TONIC_ENDPOINT";
const API_KEY: &str = "OTLP_TONIC_X_HONEYCOMB_TEAM";

fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    let endpoint = var(ENDPOINT).unwrap_or_else(|_| panic!("Bad env var {}.", ENDPOINT));
    let endpoint = Url::parse(&endpoint).expect("endpoint is not a valid url");

    let api_key = var(API_KEY).unwrap_or_else(|_| panic!("Bad env var {}.", API_KEY));

    let mut metadata = MetadataMap::new();

    metadata.insert(
        MetadataKey::from_str("x-honeycomb-team").unwrap(),
        api_key.parse().unwrap(),
    );

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(opentelemetry::sdk::trace::config().with_resource(
            opentelemetry::sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                "lab_honeycomb_service",
            )]),
        ))
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint.as_str())
                .with_metadata(metadata)
                .with_tls_config(
                    ClientTlsConfig::new().domain_name(
                        endpoint
                            .host_str()
                            .expect("the specified endpoint should have a valid host"),
                    ),
                ),
        )
        .install_batch(opentelemetry::runtime::TokioCurrentThread)
}

async fn hello() -> &'static str {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    dotenv::dotenv().ok();

    let tracer = init_tracer()?;

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default().with(telemetry);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to install tracing subscriber.");

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(web::resource("/hello").to(hello))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;

    opentelemetry::global::shutdown_tracer_provider();

    println!("Shutting down.");

    Ok(())
}
