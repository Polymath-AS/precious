pub mod cognitive;
pub mod container_app;
pub mod container_registry;
pub mod front_door;
pub mod front_door_waf;
pub mod key_vault;
pub mod log_analytics;
pub mod managed_redis;
pub mod monitor_alert;
pub mod monitor_query_alert;
pub mod network_watcher_flow_log;
pub mod postgresql;
pub mod private_dns_zone;
pub mod private_endpoint;
pub mod security_center;
pub mod storage_account;

use crate::provider::Provider;
use crate::registry::Registry;
use precious_core::resource::Cloud;
use precious_pricing::client::{AzurePricingClient, PricingClient};

pub struct AzureProvider {
    pricing: AzurePricingClient,
}

impl AzureProvider {
    pub fn new() -> Self {
        Self {
            pricing: AzurePricingClient::new(),
        }
    }
}

impl Default for AzureProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for AzureProvider {
    fn cloud(&self) -> Cloud {
        Cloud::Azure
    }

    fn pricing_client(&self) -> &dyn PricingClient {
        &self.pricing
    }

    fn register(&self, registry: &mut Registry) {
        registry.register(Box::new(postgresql::PostgresqlFlexibleServerModel));
        registry.register(Box::new(container_app::ContainerAppModel));
        registry.register(Box::new(managed_redis::ManagedRedisModel));
        registry.register(Box::new(front_door::FrontDoorProfileModel));
        registry.register(Box::new(storage_account::StorageAccountModel));
        registry.register(Box::new(container_registry::ContainerRegistryModel));
        registry.register(Box::new(log_analytics::LogAnalyticsWorkspaceModel));
        registry.register(Box::new(key_vault::KeyVaultModel));
        registry.register(Box::new(private_endpoint::PrivateEndpointModel));
        registry.register(Box::new(private_dns_zone::PrivateDnsZoneModel));
        registry.register(Box::new(cognitive::CognitiveDeploymentModel));
        registry.register(Box::new(front_door_waf::FrontDoorWafModel));
        registry.register(Box::new(security_center::SecurityCenterPricingModel));
        registry.register(Box::new(monitor_alert::MonitorMetricAlertModel));
        registry.register(Box::new(monitor_query_alert::MonitorQueryAlertModel));
        registry.register(Box::new(
            network_watcher_flow_log::NetworkWatcherFlowLogModel,
        ));

        registry.register_free(&[
            "azurerm_resource_group",
            "azurerm_virtual_network",
            "azurerm_subnet",
            "azurerm_network_security_group",
            "azurerm_subnet_network_security_group_association",
            "azurerm_role_assignment",
            "azurerm_private_dns_zone_virtual_network_link",
            "azurerm_cdn_frontdoor_endpoint",
            "azurerm_cdn_frontdoor_origin_group",
            "azurerm_cdn_frontdoor_origin",
            "azurerm_cdn_frontdoor_route",
            "azurerm_cdn_frontdoor_security_policy",
            "azurerm_monitor_action_group",
            "azurerm_monitor_diagnostic_setting",
            "azurerm_storage_container",
            "azurerm_postgresql_flexible_server_configuration",
            "azurerm_postgresql_flexible_server_firewall_rule",
            "azurerm_postgresql_flexible_server_database",
            "azurerm_network_watcher",
            "azurerm_security_center_contact",
            "azurerm_application_insights_workbook",
            "azurerm_container_app_environment",
            "azurerm_cognitive_account",
        ]);
    }
}
