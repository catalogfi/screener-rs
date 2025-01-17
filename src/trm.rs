use async_trait::async_trait;
use bon::Builder;
use chrono::TimeDelta;
use eyre::{OptionExt, Result};
use moka::future::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::interface::{AddressInfo, Screener, ScreenerResponse};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressScreeningResponse {
    pub address_risk_indicators: Vec<AddressRiskIndicator>,
    pub address: String,
    pub address_submitted: String,
    pub entities: Vec<Entity>,
    pub chain: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressRiskIndicator {
    pub category: String,
    pub category_id: String,
    pub category_risk_score_level: i32,
    pub category_risk_score_level_label: String,
    pub risk_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub category: String,
    pub category_id: String,
    pub confidence_score_label: String,
    pub entity: String,
    pub risk_score_level: i32,
    pub risk_score_level_label: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrmScreenerApiRequest {
    address: String,
    chain: String,
    account_external_id: String,
}

impl From<&AddressInfo> for TrmScreenerApiRequest {
    fn from(value: &AddressInfo) -> Self {
        Self {
            account_external_id: format!("{}_{}", value.address, value.chain),
            address: value.address.clone(),
            chain: value.chain.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CacheRecord<T> {
    record: T,
    timestamp: i64,
}

impl<T> CacheRecord<T> {
    pub fn new(record: T) -> Result<Self> {
        Ok(Self {
            record,
            timestamp: chrono::Utc::now()
                .checked_add_signed(TimeDelta::hours(24))
                .ok_or_eyre(eyre::eyre!("invalid addition to current time"))?
                .timestamp(),
        })
    }

    pub fn validate(&self) -> Result<()> {
        let time_now = chrono::Utc::now().timestamp();
        if time_now > self.timestamp {
            return Err(eyre::eyre!("expired"));
        }
        Ok(())
    }
}

#[derive(Builder, Clone)]
pub struct TrmScreener {
    url: String,
    api_key: String,
    batch_size: usize,
    risk_score_limit: i32,
    cache: Cache<String, CacheRecord<bool>>,
}

impl TrmScreener {
    async fn check_in_cache(&self, addresses: &[AddressInfo]) -> Result<Vec<AddressInfo>> {
        let mut maybe_blacklisted_address = Vec::new();
        for address_info in addresses {
            if self.cache.get(&address_info.id()).await.is_none() {
                maybe_blacklisted_address.push(address_info.clone());
            }
        }
        Ok(maybe_blacklisted_address)
    }
}

#[async_trait]
impl Screener for TrmScreener {
    async fn is_blacklisted(&self, addresses: &[AddressInfo]) -> Result<Vec<ScreenerResponse>> {
        let non_whitelisted_addresses = self.check_in_cache(addresses).await?;
        if non_whitelisted_addresses.is_empty() {
            return Ok(addresses
                .iter()
                .map(|e| ScreenerResponse {
                    address: e.clone(),
                    is_blacklisted: false,
                })
                .collect());
        }

        let client = Client::new();

        let mut all_responses = Vec::new();
        for batch in non_whitelisted_addresses.chunks(self.batch_size) {
            let inputs: Vec<TrmScreenerApiRequest> = batch.iter().map(|e| e.into()).collect();

            // Make API request
            let response = client
                .post(&self.url) // Post request to the URL
                .basic_auth(&self.api_key, Some(&self.api_key))
                .json(&inputs) // Send the inputs as JSON
                .send()
                .await? // Await the async request
                .error_for_status()? // Return an error if the response is not 200
                .json::<Vec<AddressScreeningResponse>>() // Parse response as Vec<AddressScreeningResponse>
                .await?;

            for api_resp in response {
                let mut blacklisted = false;

                // Check each entity for risk score
                for entity in &api_resp.entities {
                    if entity.risk_score_level > self.risk_score_limit {
                        blacklisted = true;
                        break;
                    }
                }

                // If not blacklisted by entities, check address risk indicators
                if !blacklisted {
                    for indicator in &api_resp.address_risk_indicators {
                        if indicator.category_risk_score_level > self.risk_score_limit {
                            blacklisted = true;
                            break;
                        }
                    }
                }

                if !blacklisted {
                    self.cache
                        .insert(
                            AddressInfo {
                                address: api_resp.address_submitted.clone(),
                                chain: api_resp.chain.clone(),
                            }
                            .id(),
                            CacheRecord::new(true)?,
                        )
                        .await;
                }

                // Map to ScreenerResponse
                let screener_response = ScreenerResponse {
                    address: AddressInfo {
                        address: api_resp.address_submitted.clone(),
                        chain: api_resp.chain,
                    },
                    is_blacklisted: blacklisted,
                };

                all_responses.push(screener_response);
            }
        }

        for non_whitelisted in non_whitelisted_addresses {
            all_responses.push(ScreenerResponse {
                address: non_whitelisted,
                is_blacklisted: false,
            });
        }

        Ok(all_responses)
    }
}

#[cfg(test)]
mod tests {

    // These tests are sanity checks only

    use std::{env, time::Duration};

    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_is_blacklisted() {
        let key = env::var("SCREENING_KEY").expect("export SCREENING_KEY in the shell");
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(200))
            .build();
        let trm_screener = TrmScreener::builder()
            .api_key(key)
            .url("https://api.trmlabs.com/public/v2/screening/addresses".to_string())
            .batch_size(5)
            .risk_score_limit(10)
            .cache(cache.clone())
            .build();

        let addresses = vec![
            // AddressInfo {
            //     chain: "bitcoin".to_string(),
            //     address: "bc1qng0keqn7cq6p8qdt4rjnzdxrygnzq7nd0pju8q".to_string(),
            // },
            AddressInfo {
                address: "0x9dd9c2d208b07bf9a4ef9ca311f36d7185749635".to_string(),
                chain: "ethereum".to_string(),
            },
        ];

        let result = trm_screener.is_blacklisted(&addresses).await.unwrap();
        dbg!(&result);

        let cached_result = cache
            .get("0x9dd9c2d208b07bf9a4ef9ca311f36d7185749635_ethereum")
            .await
            .unwrap();
        assert!(cached_result.timestamp > chrono::Utc::now().timestamp());
        assert!(!result.is_empty());
        // assert!(result[0].is_blacklisted);
    }
}
