#![allow(warnings)]

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{get, App, HttpServer, Responder};
use trm_labs::address_screener::AddressScreener;
use trm_labs::cache::TrmScreenerCache;
use trm_labs::trm::TrmScreener;

// imports
use std::error::Error;
use std::env;
use std::sync::Arc;

#[get("/")]
async fn index() -> impl Responder {
    "server index route hit"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    // create a trm screener as app state
    let trm_screener = get_address_screener().await?; 

    // creatng the app state with the trm screener
    let app_state = Data::new(trm_screener);

    // Starting the server
    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(app_state.clone()))
            .wrap(
                Cors::default() // Allows all origins
                    .allow_any_origin() // Allows all origins
                    .allow_any_method() // Allows all HTTP methods (GET, POST, etc.)
                    .allow_any_header(), // Allows all headers
            )
            .service(index)
            .service(trm_labs::handlers::screener_handler)
            
    })
    .bind("0.0.0.0:3003")?
    .run();

    println!("Server running at http://0.0.0.0:3003");
    server.await?;

    Ok(())
}

// Funtion to get the address screener
async fn get_address_screener() -> Result<AddressScreener<TrmScreener, TrmScreenerCache>, Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // create a trm screener cache
    let screener_cache = Arc::new(
            TrmScreenerCache::from_psql_url(&database_url)
                .await
                .unwrap(),
        );

        // create a trm screener
        let key = env::var("SCREENING_KEY").expect("export SCREENING_KEY in the shell");
        let trm_screener = Arc::new(
            TrmScreener::builder()
                .api_key(key)
                .url("https://api.trmlabs.com/public/v2/screening/addresses".to_string())
                .batch_size(5)
                .risk_score_limit(10)
                .cache(
                    moka::future::Cache::builder()
                        .max_capacity(1000)
                        .time_to_live(std::time::Duration::from_secs(7200))
                        .build(),
                )
                .build(),
        );

        //create an address screener 
        let address_screener = AddressScreener::new(trm_screener, screener_cache);

     
    Ok(address_screener)
}