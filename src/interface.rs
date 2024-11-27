use async_trait::async_trait;
use eyre::Result;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddressInfo {
    pub chain: String,
    pub address: String,
}

impl AddressInfo {
    pub fn id(&self) -> String {
        format!("{}_{}", self.address, self.chain)
    }
}

#[derive(Debug)]
pub struct ScreenerResponse {
    pub address: AddressInfo,
    pub is_blacklisted: bool,
}

#[async_trait]
pub trait Screener {
    async fn is_blacklisted(&self, addresses: &[AddressInfo]) -> Result<Vec<ScreenerResponse>>;
}

#[derive(Clone, Debug)]
pub struct ScreenerCacheResponse {
    pub address: AddressInfo,
    pub is_blacklisted: bool,
    pub not_found: bool,
}

impl From<ScreenerCacheResponse> for ScreenerResponse {
    fn from(val: ScreenerCacheResponse) -> Self {
        ScreenerResponse {
            address: val.address,
            is_blacklisted: val.is_blacklisted,
        }
    }
}

#[async_trait]
pub trait ScreenerCache {
    async fn is_blacklisted(&self, addresses: &[AddressInfo])
        -> Result<Vec<ScreenerCacheResponse>>;
    async fn mark_blacklisted(&self, addresses: &[ScreenerResponse]) -> Result<()>;
}
