use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use rand::Rng;
use std::{convert::Infallible, net::SocketAddr,env};
use tracing::Instrument;
use tracing::{instrument, Level};
use tracing::Span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use std::collections::HashMap;
use opentelemetry::{ global, KeyValue, Context};
use opentelemetry_sdk::{trace as sdktrace, resource::Resource};
use opentelemetry_otlp::WithExportConfig;

//Used in propagations
use opentelemetry_sdk::propagation::TraceContextPropagator;

// Utility function to extract the context from the incoming request headers
fn extract_context_from_request(req: &Request<Body>) -> Context {
    // convert the req headers to a hashmap
    let headers = req.headers().iter().map(|(k, v)| {
        (k.as_str().to_string(), v.to_str().unwrap().to_string())
    }).collect::<HashMap<String, String>>();

    global::get_text_map_propagator(|propagator| {
        propagator.extract(&headers)
    })
}

// Separate async function for the handle endpoint
#[instrument(fields(app.dice_roll))]
async fn handle_rolldice(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let random_number = rand::thread_rng().gen_range(1..7);
    Span::current().record("app.dice_roll", &random_number);
    let res = Response::new(Body::from(random_number.to_string()));
    Ok(res)
}
async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // Extract the context from the incoming request headers
    let parent_cx = extract_context_from_request(&req);

    let response =  {
        let handle_request_span = tracing::span!(Level::INFO, "handle_request", http.path=%req.uri().path(), http.method=req.method().as_str());
        handle_request_span.set_parent(parent_cx.clone());

        match (req.method(), req.uri().path()) {
            (&Method::GET, "/rolldice") => {
                //This allows us to match the parent span to the underlying function
                //See other examples at  https://docs.rs/tracing/latest/tracing/struct.Span.html#method.enter
                async move {
                    handle_rolldice(req).await
                }.instrument(handle_request_span).await

            }
            _ => {
                
                let mut not_found = Response::default();
                *not_found.status_mut() = StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        }
    };
   
    response
}

fn init_tracer()  {
    global::set_text_map_propagator(TraceContextPropagator::new());
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

    // setup tracing crate subscriber
    match tracer {
        Ok(tracer) => {
            // Create a tracing layer with the configured tracer
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            // Use the tracing subscriber `Registry`, or any other subscriber
            // that impls `LookupSpan`
            let subscriber = Registry::default().with(telemetry);
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
        Err(e) => { println!("Error setting up tracer: {:?}", e); }
    }
}


#[tokio::main]
async fn main() {
    // Setup tracing and export to Honeycomb
    init_tracer();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }
}
