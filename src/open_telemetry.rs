use futures_util::{Stream, StreamExt as _};
use opentelemetry::global;
use opentelemetry::global::shutdown_tracer_provider;
use opentelemetry::sdk::trace::Config;
use opentelemetry::sdk::{metrics::PushController, trace as sdktrace, Resource};
use opentelemetry::trace::TraceError;
use opentelemetry::{
    baggage::BaggageExt,
    metrics::ObserverResult,
    trace::{TraceContextExt, Tracer},
    Context, Key, KeyValue,
};
use std::error::Error;
use std::time::Duration;

fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    opentelemetry_jaeger::new_pipeline()
        .with_service_name("trace-demo")
        .with_trace_config(Config::default().with_resource(Resource::new(vec![
            KeyValue::new("service.name", "new_service"),
            KeyValue::new("exporter", "otlp-jaeger"),
        ])))
        .install_batch(opentelemetry::runtime::Tokio)
}