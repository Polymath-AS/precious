use crate::money::{BillingPeriod, Money};
use crate::resource::ResourceAddress;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostComponent {
    pub name: SmolStr,
    pub unit: BillingPeriod,
    pub quantity: Decimal,
    pub unit_price: Money,
    pub monthly_cost: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCost {
    pub address: ResourceAddress,
    pub resource_type: SmolStr,
    pub components: Vec<CostComponent>,
    pub monthly_total: Money,
}

impl ResourceCost {
    pub fn new(
        address: ResourceAddress,
        resource_type: impl Into<SmolStr>,
        components: Vec<CostComponent>,
    ) -> Self {
        let monthly_total = components.iter().map(|c| c.monthly_cost).sum();
        Self {
            address,
            resource_type: resource_type.into(),
            components,
            monthly_total,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakdown {
    pub resources: Vec<ResourceCost>,
    pub total_monthly_cost: Money,
}

impl Breakdown {
    pub fn new(resources: Vec<ResourceCost>) -> Self {
        let total_monthly_cost = resources.iter().map(|r| r.monthly_total).sum();
        Self {
            resources,
            total_monthly_cost,
        }
    }

    pub fn sort(&mut self, reverse: bool) {
        self.resources
            .sort_by(|a, b| a.monthly_total.amount.cmp(&b.monthly_total.amount));
        if reverse {
            self.resources.reverse();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    Added(ResourceCost),
    Removed(ResourceCost),
    Modified {
        before: ResourceCost,
        after: ResourceCost,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub changes: Vec<Change>,
    pub total_before: Money,
    pub total_after: Money,
    pub delta: Money,
}

impl Diff {
    pub fn compute(before: &Breakdown, after: &Breakdown) -> Self {
        let mut changes = Vec::new();
        let before_map: indexmap::IndexMap<String, &ResourceCost> = before
            .resources
            .iter()
            .map(|r| (r.address.to_string(), r))
            .collect();
        let after_map: indexmap::IndexMap<String, &ResourceCost> = after
            .resources
            .iter()
            .map(|r| (r.address.to_string(), r))
            .collect();

        for (addr, before_cost) in &before_map {
            match after_map.get(addr) {
                Some(after_cost) => {
                    if before_cost.monthly_total != after_cost.monthly_total {
                        changes.push(Change::Modified {
                            before: (*before_cost).clone(),
                            after: (*after_cost).clone(),
                        });
                    }
                }
                None => changes.push(Change::Removed((*before_cost).clone())),
            }
        }

        for (addr, after_cost) in &after_map {
            if !before_map.contains_key(addr) {
                changes.push(Change::Added((*after_cost).clone()));
            }
        }

        Self {
            changes,
            total_before: before.total_monthly_cost,
            total_after: after.total_monthly_cost,
            delta: after.total_monthly_cost - before.total_monthly_cost,
        }
    }
}
