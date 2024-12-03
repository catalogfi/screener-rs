use actix_web::{post, web, Responder};

use crate::{address_screener::AddressScreener, cache::TrmScreenerCache, interface::AddressInfo, trm::TrmScreener};


#[post("/screening/addresses")]
async fn screener_handler(address_screener: web::Data<AddressScreener<TrmScreener,TrmScreenerCache>>, addresses: web::Json<Vec<AddressInfo>>) -> impl Responder {
    
    let info = address_screener.is_blacklisted(&addresses).await;
    match info {
        Ok(result) => web::Json(serde_json::json!({ "result": result })),
        Err(e) => {
            let error_message = format!("Error: {:?}", e);
            web::Json(serde_json::json!({ "error": error_message }))
        }
    }
    
}