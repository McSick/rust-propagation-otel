use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server, Method, StatusCode};
use rand::Rng;
use std::{convert::Infallible, net::SocketAddr,env};
use tracing::Instrument;
use tracing::{instrument, Level};
use tracing::Span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use std::str::FromStr;
use std::collections::HashMap;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use opentelemetry::{ global, KeyValue, Context};
use opentelemetry_sdk::{trace as sdktrace, resource::Resource};
use opentelemetry_otlp::WithExportConfig;

//Used in propagations
use opentelemetry_sdk::propagation::TraceContextPropagator;
use std::io::{self, Read};
use reqwest;

fn headermap_from_hashmap<'a, I, S>(headers: I) -> HeaderMap
where
    I: Iterator<Item = (S, S)> + 'a,
    S: AsRef<str> + 'a,
{
    headers
        .map(|(name, val)| (HeaderName::from_str(name.as_ref()), HeaderValue::from_str(val.as_ref())))
        // We ignore the errors here. If you want to get a list of failed conversions, you can use Iterator::partition 
        // to help you out here
        .filter(|(k, v)| k.is_ok() && v.is_ok())
        .map(|(k, v)| (k.unwrap(), v.unwrap()))
        .collect() 
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
            "dice_client",
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
#[instrument(parent=Span::current())]
async fn get_random_number_from_server() -> Result<u32, reqwest::Error> {
    let url = "http://localhost:8080/rolldice";

    let client = reqwest::Client::new();
    let response_text = client.get(url).headers(global::get_text_map_propagator(|propagator| {
        let mut headers = HashMap::new();
        let cx = Span::current().context();
        propagator.inject_context(&cx, &mut headers);
        headermap_from_hashmap(headers.iter())
    })).send().await?.text().await?;
    let guess =response_text.parse::<u32>().unwrap();
    Ok(guess)
}
#[instrument]
fn get_user_guess() -> Result<u32, &'static str> {
    // Parse the input as an integer

    let mut input = String::new();
    println!("Enter a number between 1 and 6");
    io::stdin().read_line(&mut input).expect("Failed to read input");
    let number: Result<u32,  &'static str> = match input.trim().parse() {
        Ok(num @ 1..=6) => Ok(num),
        Ok(_) => {
            println!("Number out of Range, Please enter a number between 1 and 6.");
            Err("Number out of Range")
        }
        Err(_) => {
            println!("Invalid Character, Please enter a number between 1 and 6.");
            Err("Invalid Character")
        }
    };
    number
}
#[instrument]
fn get_should_continue() -> Result<bool, &'static str> { 
    let mut input = String::new();
    println!("Try again? (y/n)");
    io::stdin().read_line(&mut input).expect("Failed to read input");
    let should_continue: Result<bool,  &'static str> = match input.trim().to_lowercase().as_str() {
        "y" => Ok(true),
        "n" => Ok(false),
        _ => {
            //println!("Invalid Character, Please enter y or n.");
            Err("Invalid Character")
        }
    };
    should_continue
}
#[tokio::main]
async fn main() {
    // Setup tracing and export to Honeycomb
    init_tracer();
    println!("Welcome to the dice game!");
    println!("You will be asked to guess a number between 1 and 6.");
    println!("If you guess correctly, you win!");
    println!("If you guess incorrectly, you lose!");
    println!("Good luck!");

    println!("");
    // a loop to keep the program running
    loop {
        // Call the function to get the user input
       
        let game_span = tracing::span!(Level::INFO, "game_span");

        let input = game_span.in_scope(get_user_guess);


        match input {
            Ok(number) => {

                let backend_roll = async move {
                    let backend_roll = get_random_number_from_server().await;
                    backend_roll
                }.instrument(game_span.clone()).await;
                
                match backend_roll {
                    Ok(backend_roll) => {
                        println!("The server rolled a {}", backend_roll);
                        if number == backend_roll {
                            println!("You win!");
                        } else {
                            println!("You lose!");
                        }
                    }
                    Err(e) => {
                        println!("Error getting random number from server: {:?}", e);
                    }
                }

            }
            Err(_) => {
                continue;
            }
        }
        let should_continue = game_span.in_scope(get_should_continue);
        match should_continue {
            Ok(should_continue) => {
                if !should_continue {
                    break;
                }
            }
            Err(_) => {
                break;
            }
        }
    }

}

