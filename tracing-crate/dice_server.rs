use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use rand::Rng;
use std::{convert::Infallible, net::SocketAddr, env};

use tracing::{instrument, Level};
use tracing::{error, span};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

use std::collections::HashMap;
use opentelemetry::trace::TraceError;
use opentelemetry::{ global, KeyValue};
use opentelemetry_sdk::{trace as sdktrace, resource::Resource};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry::trace::{SpanKind, Status, Tracer, Span};

#[instrument]
async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/rolldice") => {
            let random_number = rand::thread_rng().gen_range(1..7);
            tracing::Span::current().record("dice_roll", &random_number);
            *response.body_mut() = Body::from(random_number.to_string());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(response)
}

fn init_tracer() {
    let honeycomb_api_key = match env::var("HONEYCOMB_API_KEY" ) {
        Ok(val) => val,
        Err(_) => "".to_string(),
    };

    let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(
        opentelemetry_otlp::new_exporter()
            .http()
            .with_endpoint("https://api.honeycomb.io")
            .with_headers(HashMap::from([
                ("x-honeycomb-team".into(), honeycomb_api_key),
          ])),
    )
    .with_trace_config(
        sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            "dice_server",
        )])),
    )
    .install_batch(opentelemetry_sdk::runtime::Tokio);
    match tracer {
        Ok(tracer) => {
            // Create a tracing layer with the configured tracer
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            // Use the tracing subscriber `Registry`, or any other subscriber
            // that impls `LookupSpan`
            let subscriber = Registry::default().with(telemetry);
            tracing::subscriber::set_global_default(subscriber);


        }
        Err(e) => {}
    }
}

#[tokio::main]
async fn main() {
    init_tracer();
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }
}
