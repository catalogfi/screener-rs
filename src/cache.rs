use async_trait::async_trait;
use eyre::Result;
use sqlx::{Pool, Postgres, Row};

use crate::interface::{AddressInfo, ScreenerCache, ScreenerCacheResponse, ScreenerResponse};

#[derive(Clone)]
pub struct TrmScreenerCache {
    db: Pool<Postgres>,
}

fn create_table_query() -> String {
    "CREATE TABLE IF NOT EXISTS blacklisted(
        address TEXT UNIQUE NOT NULL,
        chain TEXT NOT NULL
    )
    "
    .to_string()
}

impl TrmScreenerCache {
    pub async fn new(db: Pool<Postgres>) -> Result<Self> {
        sqlx::query(&create_table_query()).execute(&db).await?;

        Ok(Self { db })
    }

    pub async fn from_psql_url(url: &str) -> Result<Self> {
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(100)
            .connect(url)
            .await?;
        Self::new(db).await
    }
}

#[async_trait]
impl ScreenerCache for TrmScreenerCache {
    async fn is_blacklisted(
        &self,
        addresses: &[AddressInfo],
    ) -> Result<Vec<ScreenerCacheResponse>> {
        let params: Vec<String> = addresses.iter().map(|e| e.address.clone()).collect();
        let query = sqlx::query("SELECT address FROM blacklisted WHERE address = ANY($1)")
            .bind(&params[..])
            .fetch_all(&self.db)
            .await?;

        let existing_addresses: Vec<String> = query
            .iter()
            .map(|row| row.get::<String, _>("address"))
            .collect();

        Ok(addresses
            .iter()
            .map(|address_info| {
                let is_blacklisted = existing_addresses.contains(&address_info.address);

                ScreenerCacheResponse {
                    address: address_info.clone(),
                    is_blacklisted,
                    not_found: !is_blacklisted,
                }
            })
            .collect())
    }
    async fn mark_blacklisted(&self, addresses: &[ScreenerResponse]) -> Result<()> {
        let mut tx = self.db.begin().await?;

        for screener_response in addresses.iter().filter(|addr| addr.is_blacklisted) {
            sqlx::query(
                "INSERT INTO blacklisted (address, chain) VALUES ($1, $2) 
                ON CONFLICT (address) DO NOTHING",
            )
            .bind(&screener_response.address.address)
            .bind(&screener_response.address.chain)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
