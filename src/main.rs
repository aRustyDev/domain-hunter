mod web_driver;
mod util;
mod util::tracing;

use crate::util::tracing::otel::init_tracer_provider;
use crate::web_driver::expired_domains::*;
// use util::bad_words::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let tracer_provider = init_tracer_provider().expect("Failed to initialize tracer provider.");
    global::set_tracer_provider(tracer_provider.clone());

    let tracer = global::tracer("tracing-jaeger");
    tracer.in_span("main", |cx| {
        let span = cx.span();
        span.set_attribute(KeyValue::new("my-attribute", "my-value"));
        span.add_event(
            "Main span event".to_string(),
            vec![KeyValue::new("foo", "1")],
        );
        tracer.in_span("child-operation...", |cx| {
            let span = cx.span();
            span.add_event("Sub span event", vec![KeyValue::new("bar", "1")]);
        });
    });

    // let tracer = global::tracer("domain-hunter");
    // let mut span = tracer.start("main");

    // let bad_words = get_bad_words(BadWordSource::File).unwrap();
    println!("{:?}", basically_selenium(CrawlTarget::ExpiredDomainsDotCom).await.unwrap());


    tracer_provider.shutdown()?;
    span.end();
}
