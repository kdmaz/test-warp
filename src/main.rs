#![warn(clippy::all)]
mod profanity;
mod routes;
mod store;
mod types;
use std::env;

use config::Config;
use dotenv::dotenv;
use handle_errors::return_error;
use routes::{
    answer::add_answer,
    question::{add_question, delete_question, get_questions, update_question},
};
use store::Store;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{http::Method, Filter};

#[derive(Debug, Default, serde::Deserialize, PartialEq)]
struct Args {
    log_level: String,
    database_host: String,
    database_port: u16,
    database_name: String,
    app_port: u16,
    database_username: String,
    database_password: String,
}

#[tokio::main]
async fn main() -> Result<(), handle_errors::Error> {
    let config = Config::builder()
        .add_source(config::File::with_name("setup"))
        .build()
        .unwrap();
    let config = config.try_deserialize::<Args>().unwrap();

    dotenv().ok();

    if env::var("BAD_WORDS_API_KEY").is_err() {
        panic!("BAD_WORDS_API_KEY not set");
    }

    if env::var("PASETO_KEY").is_err() {
        panic!("PASETO_KEY not set");
    }

    let port = std::env::var("PORT")
        .ok()
        .map(|val| val.parse::<u16>())
        .unwrap_or(Ok(config.app_port))
        .map_err(handle_errors::Error::ParseError)?;

    let log_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        format!(
            "handle_errors={},test_warp={},warp={}",
            config.log_level, config.log_level, config.log_level
        )
    });

    tracing_subscriber::fmt()
        // Use the filter we built above to determine which traces to record.
        .with_env_filter(log_filter)
        // Record an event when each span closes. This can be used to time our
        // routes' durations!
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(&[Method::PUT, Method::DELETE, Method::GET, Method::POST]);

    let store = Store::new(&format!(
        "postgres://{username}:{password}@{host}:{port}/{name}",
        username = config.database_username,
        password = config.database_password,
        host = config.database_host,
        port = config.database_port,
        name = config.database_name
    ))
    .await
    .map_err(handle_errors::Error::DatabaseQueryError)?;

    sqlx::migrate!()
        .run(&store.clone().pool)
        .await
        .map_err(handle_errors::Error::MigrationError)?;

    let store_filter = warp::any().map(move || store.clone());

    let get_questions = warp::get()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(warp::query())
        .and(store_filter.clone())
        .and_then(get_questions);

    let add_question = warp::post()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::json())
        .and_then(add_question);

    let update_question = warp::put()
        .and(warp::path("questions"))
        .and(warp::path::param::<i32>())
        .and(warp::path::end())
        .and(routes::authentication::auth())
        .and(store_filter.clone())
        .and(warp::body::json())
        .and_then(update_question);

    let delete_question = warp::delete()
        .and(warp::path("questions"))
        .and(warp::path::param::<i32>())
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(delete_question);

    let add_answer = warp::post()
        .and(warp::path("answers"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::form())
        .and_then(add_answer);

    let registration = warp::post()
        .and(warp::path("registration"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::json())
        .and_then(routes::authentication::register);

    let login = warp::post()
        .and(warp::path("login"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::json())
        .and_then(routes::authentication::login);

    let routes = get_questions
        .or(add_question)
        .or(update_question)
        .or(delete_question)
        .or(add_answer)
        .or(registration)
        .or(login)
        .with(cors)
        .with(warp::trace::request())
        .recover(return_error);

    warp::serve(routes).run(([127, 0, 0, 1], port)).await;

    Ok(())
}
