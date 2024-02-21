use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use rand::Rng;
use std::{convert::Infallible, net::SocketAddr,env};

use std::collections::HashMap;
use opentelemetry::trace::TraceError;
use opentelemetry::{ global, KeyValue, Context};
use opentelemetry_sdk::{trace as sdktrace, resource::Resource};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry::trace::{Status, Tracer, Span, FutureExt, TraceContextExt};
use opentelemetry_semantic_conventions::trace;

//Used in propagations
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::TracerProvider};


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
async fn handle_rolldice(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let tracer = global::tracer("dice_server");
    let mut span = tracer
        .span_builder("rolldice")
        .with_kind(SpanKind::Internal)
        .start(&tracer);
    span.add_event("We did it", vec![]);
    let random_number = rand::thread_rng().gen_range(1..7);
    span.set_attribute(KeyValue::new("dice_roll", random_number));
    let res = Response::new(Body::from(random_number.to_string()));
    Ok(res)
}
async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // Extract the context from the incoming request headers
    let parent_cx = extract_context_from_request(&req);


    let mut response =  {
        let tracer = global::tracer("dice_server");

        let mut span = tracer.span_builder(format!("{} {}", req.method(), req.uri().path())).with_kind(SpanKind::Server).start_with_context(&tracer, &parent_cx);
        let cx = Context::default().with_span(span);
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/rolldice") => {
                handle_rolldice(req).with_context(cx).await
            }
            _ => {
                cx.span()
                .set_attribute(KeyValue::new(trace::HTTP_RESPONSE_STATUS_CODE, 404));
                let mut not_found = Response::default();
                *not_found.status_mut() = StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        }
    };
   
    response
}

fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let honeycomb_api_key = match env::var("HONEYCOMB_API_KEY" ) {
        Ok(val) => val,
        Err(_) => "".to_string(),
    };

    opentelemetry_otlp::new_pipeline()
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
    .install_batch(opentelemetry_sdk::runtime::Tokio)
}


#[tokio::main]
async fn main() {
    // Setup tracing and export to Honeycomb
    let _ = init_tracer();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on {addr}");
    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }
}
