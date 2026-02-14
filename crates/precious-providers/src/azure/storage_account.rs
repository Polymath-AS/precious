use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::{State, TfResource};
use precious_pricing::client::PricingClient;
use precious_pricing::types::PriceQuery;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

pub struct StorageAccountModel;

impl StorageAccountModel {
    /// Map Terraform `account_kind` to Azure Retail Prices API `productName`.
    fn product_name(account_kind: &str, account_tier: &str) -> &'static str {
        match (account_kind, account_tier) {
            ("BlobStorage", _) => "Blob Storage",
            ("BlockBlobStorage", _) => "Premium Block Blob",
            ("FileStorage", _) => "Files v2",
            ("Storage", _) => "General Block Blob",
            // StorageV2 (default) and anything else
            (_, "Premium") => "Premium Block Blob",
            _ => "General Block Blob v2",
        }
    }

    /// Determine the access tier label used in Azure pricing SKU/meter names.
    /// Premium accounts don't use Hot/Cool — they use "Premium" directly.
    fn access_tier_label(account_tier: &str, access_tier: &str) -> &'static str {
        match account_tier {
            "Premium" => "Premium",
            _ => match access_tier {
                "Cool" => "Cool",
                "Cold" => "Cold",
                _ => "Hot",
            },
        }
    }

    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let account_tier = resource.get_string("account_tier").unwrap_or("Standard");
        let replication_type = resource
            .get_string("account_replication_type")
            .unwrap_or("LRS");
        let account_kind = resource.get_string("account_kind").unwrap_or("StorageV2");
        let access_tier = resource.get_string("access_tier").unwrap_or("Hot");

        let region =
            resource
                .get_string("location")
                .ok_or_else(|| PreciousError::MissingField {
                    resource: resource.address.to_string(),
                    field: "location".to_string(),
                })?;

        let tier_label = Self::access_tier_label(account_tier, access_tier);
        let product_name = Self::product_name(account_kind, account_tier);

        // Azure API uses "{tier} {replication}" for skuName (e.g. "Hot LRS")
        let sku_name = format!("{tier_label} {replication_type}");
        // and "{tier} {replication} Data Stored" for meterName
        let meter_name = format!("{tier_label} {replication_type} Data Stored");

        let query = PriceQuery::azure("Storage", region)
            .filter("skuName", &sku_name)
            .filter("meterName", &meter_name)
            .filter("productName", product_name)
            .filter("priceType", "Consumption");

        let unit_price = pricing.query_price(&query).await.map_err(|e| {
            PreciousError::PricingError(format!(
                "failed to get storage price for {sku_name}: {e}"
            ))
        })?;

        Ok(vec![CostComponent {
            name: SmolStr::new(format!(
                "Data storage ({account_kind}, {tier_label} {replication_type})"
            )),
            unit: BillingPeriod::GBMonth,
            quantity: Decimal::ZERO,
            quantity_unit: SmolStr::new("GB/month"),
            unit_price: unit_price.price,
            monthly_cost: Money::zero(),
            quantity_max: None,
            monthly_cost_max: None,
        }])
    }
}

impl ResourceCostModel for StorageAccountModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_storage_account")
    }

    fn estimate<'a>(
        &'a self,
        resource: &'a TfResource,
        usage: Option<&'a UsageEntry>,
        pricing: &'a dyn PricingClient,
        _state: &'a State,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CostComponent>, PreciousError>> + Send + 'a>> {
        Box::pin(self.do_estimate(resource, usage, pricing))
    }
}
