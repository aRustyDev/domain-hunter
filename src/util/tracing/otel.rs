use opentelemetry::{
    global,
    trace::{TraceContextExt, TraceError, Tracer},
    KeyValue,
};
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;

fn init_tracer() {
    // Swap this no-op provider for your tracing service of choice (jaeger, zipkin, etc)
    let provider = NoopTracerProvider::new();

    // Configure the global `TracerProvider` singleton when your app starts
    // (there is a no-op default if this is not set by your application)
    let _ = global::set_tracer_provider(provider);
}

fn do_something_tracked() {
    // Then you can get a named tracer instance anywhere in your codebase.
    let tracer = global::tracer("my-component");

    tracer.in_span("doing_work", |cx| {
        // Traced app logic here...
    });
}

fn init_tracer_provider() -> Result<opentelemetry_sdk::trace::TracerProvider, TraceError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()?;

    Ok(TracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name("tracing-jaeger")
                .build(),
        )
        .build())
}

// in main or other app start
init_tracer();
do_something_tracked();