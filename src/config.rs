use serde::Deserialize;
use std::fs;
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct Config {
    // postgres db url
    pub db_url: String,
    // screener api key
    pub screener_api_key: String,
    // risk score limit
    pub risk_score_limit: i32,
    // whitelisted addresses
    pub whitelisted_addresses: Vec<String>,
    // limit on the number of addresses to be screened in a single request
    pub request_batch_size: usize,
}
impl Config{
    pub fn load() -> Result<Self, Box<dyn Error>> {
        // Read the configuration file
        let config_data = fs::read_to_string("config.json");
        if let Err(e) = config_data {
            println!("Error reading the configuration file: {:?}", e);
            return Err(Box::new(e));
        }
        
        let config_data = config_data.unwrap();
        // Parse the JSON into the Config struct
        let config: Config = serde_json::from_str(&config_data)?;
    
        // Print the configuration to verify
        // println!("Configuration loaded: {:?}", config);
    
        Ok(config)
    }
}
