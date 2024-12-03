use crate::interface::{
    AddressInfo, Screener, ScreenerCache, ScreenerCacheResponse, ScreenerResponse,
};
use eyre::Result;
use std::sync::Arc;

pub struct AddressScreener<T: Screener, S: ScreenerCache> {
    screener: Arc<T>,
    screener_cache: Arc<S>,
}

impl<T: Screener, S: ScreenerCache> AddressScreener<T, S> {
    pub fn new(screener: Arc<T>, screener_cache: Arc<S>) -> Self {
        Self {
            screener,
            screener_cache,
        }
    }
    pub async fn is_blacklisted(&self, addresses: &[AddressInfo]) -> Result<Vec<ScreenerResponse>> {
        // Step:1 Check in DB, if we find any blacklisted addresses
        let response = self.screener_cache.is_blacklisted(addresses).await?;

        // Step:2 filter by not_found
        let not_found_addresses: Vec<ScreenerCacheResponse> = response
            .clone()
            .into_iter()
            .filter(|res| res.not_found)
            .collect();

        dbg!(&not_found_addresses);
        // Step:3 If empty, then we found all addresses, hence return the response
        if not_found_addresses.is_empty() {
            return Ok(response.into_iter().map(|v| v.into()).collect());
        }

        // we did not find only few items
        if addresses.len() != not_found_addresses.len() {
            dbg!("Some addresses are not found  screener cache");
            // Extract the list of `AddressInfo` for the not found addresses
            let missing_addresses: Vec<AddressInfo> = not_found_addresses
                .iter()
                .map(|res| res.address.clone())
                .collect();

            dbg!(&missing_addresses);

            // Query the screener for the missing addresses
            let screener_response = self.screener.is_blacklisted(&missing_addresses).await?;

            // Mark the newly found blacklisted addresses in the cache
            self.screener_cache
                .mark_blacklisted(&screener_response)
                .await?;

            // Combine cached results (those found) and the newly fetched results
            let mut combined_response: Vec<ScreenerResponse> = response
                .into_iter()
                .filter(|res| !res.not_found)
                .map(|v| v.into())
                .collect();

            combined_response.extend(screener_response);
            Ok(combined_response)
        } else {
            dbg!("All addresses are not found in screener cache");
            // we found nothing in db
            let res = self.screener.is_blacklisted(addresses).await?;
            self.screener_cache.mark_blacklisted(&res).await?;
            Ok(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, env, sync::Arc, time::Duration};

    use moka::future::Cache;

    use crate::{
        address_screener::AddressScreener,
        cache::TrmScreenerCache,
        interface::{AddressInfo, ScreenerCache},
        trm::TrmScreener,
    };

    #[tokio::test]
    async fn test_screener() {
        let screener_cache = Arc::new(
            TrmScreenerCache::from_psql_url("postgres://postgres:root@localhost:5432/garden")
                .await
                .unwrap(),
        );
        let key = env::var("SCREENING_KEY").expect("export SCREENING_KEY in the shell");
        
        let trm_cache= Cache::builder()
                        .max_capacity(1000)
                        .time_to_live(Duration::from_secs(10))
                        .build();

        let trm_screener = Arc::new(
            TrmScreener::builder()
                .api_key(key)
                .url("https://api.trmlabs.com/public/v2/screening/addresses".to_string())
                .batch_size(5)
                .risk_score_limit(10)
                .cache(
                   trm_cache
                )
                .always_whitelisted(HashSet::new())
                .build(),
        );

        let address_screener = AddressScreener::new(trm_screener, screener_cache.clone());

        // let addresses = vec![AddressInfo {
        //     chain: "ethereum".to_string(),
        //     address: "0x699A8B34420A2a3bA1817b2C061ed852448F4170".to_string(),
        // }];
        let addresses = vec![AddressInfo {
                chain: "bitcoin".to_string(),
                address: "bc1qng0keqn7cq6p8qdt4rjnzdxrygnzq7nd0pju8q".to_string(),
        },AddressInfo {
            chain: "ethereum".to_string(),
            address: "0x699A8B34420A2a3bA1817b2C061ed852448F4170".to_string(),
        }
        ];
        
        // screener cache is empty before the test
        println!("Cache is empty before the test");
        let res = screener_cache.is_blacklisted(&addresses).await.unwrap();
        dbg!(res);

        // calling is blacklisted will fill the cache
        let info = address_screener.is_blacklisted(&addresses).await.unwrap();
        dbg!(&info);

        // cache is filled after the test
        println!("Cache is filled after the test");
        let res = screener_cache.is_blacklisted(&addresses).await.unwrap();
        dbg!(res);

    }

    #[tokio::test]
    async fn testing_only_blacklisted_addresses(){
        let screener_cache = Arc::new(
            TrmScreenerCache::from_psql_url("postgres://postgres:root@localhost:5432/garden")
                .await
                .unwrap(),
        );
        let key = env::var("SCREENING_KEY").expect("export SCREENING_KEY in the shell");
        
        let trm_cache= Cache::builder()
                        .max_capacity(1000)
                        .time_to_live(Duration::from_secs(10))
                        .build();

        let trm_screener = Arc::new(
            TrmScreener::builder()
                .api_key(key)
                .url("https://api.trmlabs.com/public/v2/screening/addresses".to_string())
                .batch_size(5)
                .risk_score_limit(10)
                .cache(
                   trm_cache
                ).always_whitelisted(HashSet::new())
                .build(),
        );

        let address_screener = AddressScreener::new(trm_screener, screener_cache.clone());

        // let addresses = vec![AddressInfo {
        //     chain: "ethereum".to_string(),
        //     address: "0x699A8B34420A2a3bA1817b2C061ed852448F4170".to_string(),
        // }];
        let addresses = vec![AddressInfo {
                chain: "bitcoin".to_string(),
                address: "bc1qng0keqn7cq6p8qdt4rjnzdxrygnzq7nd0pju8q".to_string(),
        },AddressInfo {
                chain: "bitcoin".to_string(),
                address: "bc1qng0keqn7cq6p8qdt4rjnzdxrygnzq7nd0pju8q".to_string(),
        }];

        // calling is blacklisted will fill the cache
        let info = address_screener.is_blacklisted(&addresses).await.unwrap();
        dbg!(&info);


    }
}



