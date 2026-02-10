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

        let min_replicas = resource
            .get_nested_f64(&["template", "min_replicas"])
            .map(|v| Decimal::try_from(v).unwrap_or(Decimal::ONE))
            .unwrap_or(Decimal::ONE);

        let max_replicas = resource
            .get_nested_f64(&["template", "max_replicas"])
            .and_then(|v| Decimal::try_from(v).ok());

        let has_range = max_replicas
            .is_some_and(|max| max > min_replicas);

        let vcpu_rate: Decimal = VCPU_MONTHLY_RATE
            .parse()
            .expect("constant VCPU_MONTHLY_RATE must be valid Decimal");
        let mem_rate: Decimal = MEMORY_GIB_MONTHLY_RATE
            .parse()
            .expect("constant MEMORY_GIB_MONTHLY_RATE must be valid Decimal");

        let vcpu_qty_min = cpu * min_replicas;
        let mem_qty_min = memory_gib * min_replicas;
        let vcpu_monthly_min = vcpu_rate * vcpu_qty_min;
        let mem_monthly_min = mem_rate * mem_qty_min;

        let (vcpu_qty_max, vcpu_monthly_max, mem_qty_max, mem_monthly_max) = if has_range {
            let max = max_replicas.expect("checked by has_range");
            (
                Some(cpu * max),
                Some(Money::usd(vcpu_rate * cpu * max)),
                Some(memory_gib * max),
                Some(Money::usd(mem_rate * memory_gib * max)),
            )
        } else {
            (None, None, None, None)
        };

        Ok(vec![
            CostComponent {
                name: SmolStr::new("vCPU"),
                unit: BillingPeriod::Month,
                quantity: vcpu_qty_min,
                quantity_unit: SmolStr::new("vCPUs"),
                unit_price: Money::usd(vcpu_rate),
                monthly_cost: Money::usd(vcpu_monthly_min),
                quantity_max: vcpu_qty_max,
                monthly_cost_max: vcpu_monthly_max,
            },
            CostComponent {
                name: SmolStr::new("Memory"),
                unit: BillingPeriod::Month,
                quantity: mem_qty_min,
                quantity_unit: SmolStr::new("GiB"),
                unit_price: Money::usd(mem_rate),
                monthly_cost: Money::usd(mem_monthly_min),
                quantity_max: mem_qty_max,
                monthly_cost_max: mem_monthly_max,
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
