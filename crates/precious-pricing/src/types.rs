use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::Cloud;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct PriceFilter {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuery {
    pub cloud: Cloud,
    pub service: String,
    pub region: String,
    pub filters: Vec<PriceFilter>,
}

impl PriceQuery {
    pub fn azure(service: &str, region: &str) -> Self {
        Self {
            cloud: Cloud::Azure,
            service: service.to_string(),
            region: region.to_string(),
            filters: Vec::new(),
        }
    }

    pub fn filter(mut self, field: &str, value: &str) -> Self {
        self.filters.push(PriceFilter {
            field: field.to_string(),
            value: value.to_string(),
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitPrice {
    pub unit: BillingPeriod,
    pub price: Money,
}

impl UnitPrice {
    pub fn hourly_usd(amount: Decimal) -> Self {
        Self {
            unit: BillingPeriod::Hour,
            price: Money::usd(amount),
        }
    }

    pub fn monthly_usd(amount: Decimal) -> Self {
        Self {
            unit: BillingPeriod::Month,
            price: Money::usd(amount),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PricingError {
    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("price not found for query: {0}")]
    NotFound(String),

    #[error("rate limited by pricing API")]
    RateLimited,

    #[error("cache error: {0}")]
    Cache(String),

    #[error("deserialization error: {0}")]
    Deserialize(String),
}
