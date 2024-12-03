// config
mod config;

use axum::{
    routing::{get, post},
    Json, Router, Extension,
};
use serde_json::json;
use std::{error::Error, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

// trmlabs imports
use trm_labs::address_screener::AddressScreener;
use trm_labs::cache::TrmScreenerCache;
use trm_labs::interface::AddressInfo;
use trm_labs::trm::TrmScreener;


// app state
pub struct AppState {
    address_screener: Arc<RwLock<AddressScreener<TrmScreener, TrmScreenerCache>>>,
    config: config::Config,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // laod configuration
    let config = config::Config::load()?;
    // Create the TRM screener
    let address_screener = get_address_screener(&config).await?;

    // Define the application state
    let app_state = Arc::new(AppState {
        address_screener: Arc::new(RwLock::new(address_screener)),
        config,
    });

    // Define the Axum app
    let app = Router::new()
        .route("/", get(index))
        .route("/screening/addresses", post(screener_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(Extension(app_state));

    // Define the server address and start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running at http://{}", "0.0.0.0:3000");
    axum::serve(listener, app).await?;
    Ok(())
}

// Index route
async fn index() -> &'static str {
    println!("/GET / hit");
    "server index route hit"
}

// Screening handler
async fn screener_handler(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(addresses): Json<Vec<AddressInfo>>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {

    println!("/POST /screening/addresses hit");

    // limiting the number of addresses to be screened
    if addresses.len() > app_state.config.request_batch_size  {
        // bad request
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"error": "Batch size limit exceeded"})),
        );
    }


    let addresses= remove_duplicates(addresses);
    let screener = app_state.address_screener.read().await;
    match screener.is_blacklisted(&addresses).await {
        Ok(result) => {
            let formatted_result: Vec<_> = result.into_iter().map(|entry| {
            json!({
                "address": entry.address.address,
                "chain": entry.address.chain,
                "is_blacklisted": entry.is_blacklisted
            })
            }).collect();
            (axum::http::StatusCode::OK, Json(json!(formatted_result)))
        },
        Err(e) => {
            let error_message = format!("Error: {:?}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": error_message })))
        }
    }
}

// Function to initialize the TRM screener
async fn get_address_screener(config:&config::Config) -> Result<AddressScreener<TrmScreener, TrmScreenerCache>, Box<dyn Error>> {
    let database_url = config.db_url.clone();
    let key=config.screener_api_key.clone();
    let risk_score_limit=config.risk_score_limit;
    let always_whitelisted=config.whitelisted_addresses.clone();

    // creating a always whitelisted set
    let always_whitelisted: std::collections::HashSet<String> = always_whitelisted.into_iter().collect();


    // Create the TRM screener cache
    let screener_cache = Arc::new(
        TrmScreenerCache::from_psql_url(&database_url)
            .await
            .unwrap(),
    );

    
    let trm_screener = Arc::new(
        TrmScreener::builder()
            .api_key(key)
            .url("https://api.trmlabs.com/public/v2/screening/addresses".to_string())
            .batch_size(10)
            .risk_score_limit(risk_score_limit)
            .cache(
                moka::future::Cache::builder()
                    .max_capacity(1000)
                    .time_to_live(std::time::Duration::from_secs(7200))
                    .build(),
            ).always_whitelisted(always_whitelisted)
            .build(),
    );

    // Create an address screener
    let address_screener = AddressScreener::new(trm_screener, screener_cache);

    Ok(address_screener)
}


fn remove_duplicates(addresses: Vec<AddressInfo>) -> Vec<AddressInfo> {
    let mut seen = std::collections::HashSet::new();
    addresses.into_iter().filter(|e| seen.insert(e.clone())).collect()
}