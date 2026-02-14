use crate::types::{PriceQuery, PricingError, UnitPrice};
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;

pub trait PricingClient: Send + Sync {
    fn query_price<'a>(
        &'a self,
        query: &'a PriceQuery,
    ) -> Pin<Box<dyn Future<Output = Result<UnitPrice, PricingError>> + Send + 'a>>;
}

pub struct AwsPricingClient {
    #[allow(dead_code)]
    http: reqwest::Client,
    #[allow(dead_code)]
    base_url: String,
}

impl Default for AwsPricingClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AwsPricingClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: "https://pricing.api.infracost.io".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    async fn do_query(&self, query: &PriceQuery) -> Result<UnitPrice, PricingError> {
        tracing::debug!("querying price for {:?}", query);
        Err(PricingError::NotFound(format!(
            "pricing not yet implemented for {:?}",
            query
        )))
    }
}

impl PricingClient for AwsPricingClient {
    fn query_price<'a>(
        &'a self,
        query: &'a PriceQuery,
    ) -> Pin<Box<dyn Future<Output = Result<UnitPrice, PricingError>> + Send + 'a>> {
        Box::pin(self.do_query(query))
    }
}

#[derive(Debug, Deserialize)]
struct AzureRetailPriceResponse {
    #[serde(rename = "Items")]
    items: Vec<AzureRetailPriceItem>,
    #[serde(rename = "NextPageLink")]
    #[allow(dead_code)]
    next_page_link: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzureRetailPriceItem {
    retail_price: f64,
    unit_of_measure: String,
    #[allow(dead_code)]
    meter_name: String,
    #[allow(dead_code)]
    product_name: String,
    #[allow(dead_code)]
    sku_name: String,
}

pub struct AzurePricingClient {
    http: reqwest::Client,
    base_url: String,
}

impl Default for AzurePricingClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AzurePricingClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: "https://prices.azure.com/api/retail/prices".to_string(),
        }
    }

    async fn do_query(&self, query: &PriceQuery) -> Result<UnitPrice, PricingError> {
        let mut filter_parts: Vec<String> = Vec::with_capacity(query.filters.len() + 3);

        filter_parts.push(format!("serviceName eq '{}'", query.service));

        if !query.region.is_empty() {
            filter_parts.push(format!("armRegionName eq '{}'", query.region));
        }

        for f in &query.filters {
            if f.field == "contains" {
                filter_parts.push(format!("contains({}, '{}')", f.value, f.field));
            } else {
                filter_parts.push(format!("{} eq '{}'", f.field, f.value));
            }
        }

        let filter = filter_parts.join(" and ");

        tracing::debug!("azure pricing query: $filter={filter}");

        let encoded_filter = encode_uri_component(&filter);
        let url = format!(
            "{}?api-version=2023-01-01-preview&$filter={}",
            self.base_url, encoded_filter
        );

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| PricingError::Http(e.to_string()))?;

        if resp.status() == 429 {
            return Err(PricingError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(PricingError::Http(format!(
                "Azure pricing API returned {}",
                resp.status()
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| PricingError::Http(e.to_string()))?;

        let data: AzureRetailPriceResponse =
            serde_json::from_str(&body).map_err(|e| PricingError::Deserialize(e.to_string()))?;

        let item = data.items.first().ok_or_else(|| {
            PricingError::NotFound(format!("no price found for filter: {filter}"))
        })?;

        let amount = rust_decimal::Decimal::try_from(item.retail_price)
            .map_err(|e| PricingError::Deserialize(e.to_string()))?;

        let unit = parse_unit_of_measure(&item.unit_of_measure);

        Ok(UnitPrice {
            unit,
            price: precious_core::money::Money::usd(amount),
        })
    }
}

fn encode_uri_component(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push_str(&format!("{byte:02X}"));
            }
        }
    }
    result
}

fn parse_unit_of_measure(uom: &str) -> precious_core::money::BillingPeriod {
    use precious_core::money::BillingPeriod;
    match uom {
        "1 Second" => BillingPeriod::Second,
        "1 Hour" => BillingPeriod::Hour,
        "1/Month" | "1/Day" => BillingPeriod::Month,
        "1 GB/Month" | "1 GiB/Month" => BillingPeriod::GBMonth,
        "1 GiB Second" => BillingPeriod::GiBSecond,
        _ => BillingPeriod::Month,
    }
}

impl PricingClient for AzurePricingClient {
    fn query_price<'a>(
        &'a self,
        query: &'a PriceQuery,
    ) -> Pin<Box<dyn Future<Output = Result<UnitPrice, PricingError>> + Send + 'a>> {
        Box::pin(self.do_query(query))
    }
}

pub struct PlanetScalePricingClient {
    postgres_prices: std::collections::HashMap<&'static str, rust_decimal::Decimal>,
    vitess_prices: std::collections::HashMap<&'static str, rust_decimal::Decimal>,
    storage_overage_per_gb: rust_decimal::Decimal,
}

impl Default for PlanetScalePricingClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanetScalePricingClient {
    pub fn new() -> Self {
        use rust_decimal::Decimal;

        let postgres_prices = std::collections::HashMap::from([
            ("PS-5", Decimal::new(15, 0)),
            ("PS-10", Decimal::new(34, 0)),
            ("PS-20", Decimal::new(59, 0)),
            ("PS-40", Decimal::new(99, 0)),
            ("PS-80", Decimal::new(179, 0)),
            ("PS-160", Decimal::new(349, 0)),
            ("PS-320", Decimal::new(699, 0)),
            ("PS-400", Decimal::new(999, 0)),
        ]);

        let vitess_prices = std::collections::HashMap::from([
            ("PS-10", Decimal::new(39, 0)),
            ("PS-20", Decimal::new(59, 0)),
            ("PS-40", Decimal::new(99, 0)),
            ("PS-80", Decimal::new(179, 0)),
            ("PS-160", Decimal::new(349, 0)),
            ("PS-320", Decimal::new(699, 0)),
            ("PS-400", Decimal::new(999, 0)),
        ]);

        Self {
            postgres_prices,
            vitess_prices,
            storage_overage_per_gb: Decimal::new(50, 2),
        }
    }

    async fn do_query(&self, query: &PriceQuery) -> Result<UnitPrice, PricingError> {
        let cluster_size = query
            .filters
            .iter()
            .find(|f| f.field == "clusterSize")
            .map(|f| f.value.as_str())
            .unwrap_or("PS-10");

        let price = match query.service.as_str() {
            "PlanetScale Postgres" => self.postgres_prices.get(cluster_size).copied(),
            "PlanetScale Vitess" => self.vitess_prices.get(cluster_size).copied(),
            "PlanetScale Storage" => Some(self.storage_overage_per_gb),
            _ => None,
        };

        match price {
            Some(amount) => Ok(UnitPrice {
                unit: precious_core::money::BillingPeriod::Month,
                price: precious_core::money::Money::usd(amount),
            }),
            None => Err(PricingError::NotFound(format!(
                "no PlanetScale price for service={} cluster_size={cluster_size}",
                query.service,
            ))),
        }
    }
}

impl PricingClient for PlanetScalePricingClient {
    fn query_price<'a>(
        &'a self,
        query: &'a PriceQuery,
    ) -> Pin<Box<dyn Future<Output = Result<UnitPrice, PricingError>> + Send + 'a>> {
        Box::pin(self.do_query(query))
    }
}

pub struct StaticPricingClient {
    prices: Vec<(String, UnitPrice)>,
}

impl StaticPricingClient {
    pub fn new(prices: Vec<(String, UnitPrice)>) -> Self {
        Self { prices }
    }

    async fn do_query(&self, query: &PriceQuery) -> Result<UnitPrice, PricingError> {
        let key = format!(
            "{}:{}:{}",
            query.service,
            query.region,
            query
                .filters
                .iter()
                .map(|f| format!("{}={}", f.field, f.value))
                .collect::<Vec<_>>()
                .join(",")
        );
        self.prices
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, p)| p.clone())
            .ok_or(PricingError::NotFound(key))
    }
}

impl PricingClient for StaticPricingClient {
    fn query_price<'a>(
        &'a self,
        query: &'a PriceQuery,
    ) -> Pin<Box<dyn Future<Output = Result<UnitPrice, PricingError>> + Send + 'a>> {
        Box::pin(self.do_query(query))
    }
}
