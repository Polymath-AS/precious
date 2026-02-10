use crate::registry::ResourceCostModel;
use crate::usage::UsageEntry;
use precious_core::cost::CostComponent;
use precious_core::error::PreciousError;
use precious_core::money::{BillingPeriod, Money};
use precious_core::resource::{Cloud, ResourceKind};
use precious_core::state::TfResource;
use precious_pricing::client::PricingClient;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use std::future::Future;
use std::pin::Pin;

const VCPU_MONTHLY_RATE: &str = "63.072";
const MEMORY_GIB_MONTHLY_RATE: &str = "7.884";
const MEBIBYTES_PER_GIBIBYTE: u32 = 1024;

pub struct ContainerAppModel;

fn parse_memory_gib(raw: &str) -> Result<Decimal, PreciousError> {
    if let Some(gi) = raw.strip_suffix("Gi") {
        gi.parse::<Decimal>()
            .map_err(|e| PreciousError::PricingError(format!("invalid memory value '{raw}': {e}")))
    } else if let Some(mi) = raw.strip_suffix("Mi") {
        let mib = mi.parse::<Decimal>().map_err(|e| {
            PreciousError::PricingError(format!("invalid memory value '{raw}': {e}"))
        })?;
        Ok(mib / Decimal::from(MEBIBYTES_PER_GIBIBYTE))
    } else {
        Err(PreciousError::PricingError(format!(
            "unsupported memory unit in '{raw}', expected Gi or Mi suffix"
        )))
    }
}

impl ContainerAppModel {
    async fn do_estimate(
        &self,
        resource: &TfResource,
        _usage: Option<&UsageEntry>,
        _pricing: &dyn PricingClient,
    ) -> Result<Vec<CostComponent>, PreciousError> {
        let cpu_raw = resource
            .get_nested_f64(&["template", "container", "cpu"])
            .ok_or_else(|| PreciousError::MissingField {
                resource: resource.address.to_string(),
                field: "template.container.cpu".to_string(),
            })?;
        let cpu = Decimal::try_from(cpu_raw).map_err(|e| {
            PreciousError::PricingError(format!("invalid cpu value '{cpu_raw}': {e}"))
        })?;

        let memory_str = resource
            .get_nested_str(&["template", "container", "memory"])
            .ok_or_else(|| PreciousError::MissingField {
                resource: resource.address.to_string(),
                field: "template.container.memory".to_string(),
            })?;
        let memory_gib = parse_memory_gib(memory_str)?;

        let replicas = resource
            .get_nested_f64(&["template", "min_replicas"])
            .map(|v| Decimal::try_from(v).unwrap_or(Decimal::ONE))
            .unwrap_or(Decimal::ONE);

        let vcpu_rate: Decimal = VCPU_MONTHLY_RATE
            .parse()
            .expect("constant VCPU_MONTHLY_RATE must be valid Decimal");
        let mem_rate: Decimal = MEMORY_GIB_MONTHLY_RATE
            .parse()
            .expect("constant MEMORY_GIB_MONTHLY_RATE must be valid Decimal");

        let vcpu_monthly = vcpu_rate * cpu * replicas;
        let mem_monthly = mem_rate * memory_gib * replicas;

        Ok(vec![
            CostComponent {
                name: SmolStr::new("vCPU"),
                unit: BillingPeriod::Month,
                quantity: cpu * replicas,
                unit_price: Money::usd(vcpu_rate),
                monthly_cost: Money::usd(vcpu_monthly),
            },
            CostComponent {
                name: SmolStr::new("Memory"),
                unit: BillingPeriod::Month,
                quantity: memory_gib * replicas,
                unit_price: Money::usd(mem_rate),
                monthly_cost: Money::usd(mem_monthly),
            },
        ])
    }
}

impl ResourceCostModel for ContainerAppModel {
    fn kind(&self) -> ResourceKind {
        ResourceKind::new(Cloud::Azure, "azurerm_container_app")
    }

    fn estimate<'a>(
        &'a self,
        resource: &'a TfResource,
        usage: Option<&'a UsageEntry>,
        pricing: &'a dyn PricingClient,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CostComponent>, PreciousError>> + Send + 'a>> {
        Box::pin(self.do_estimate(resource, usage, pricing))
    }
}
