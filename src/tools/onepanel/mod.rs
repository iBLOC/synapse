pub mod client;
pub mod monitoring;
pub mod websites;
pub mod apps;
pub mod containers;
pub mod logs;
pub mod cronjobs;
pub mod task_center;
pub mod nodes;

pub use client::{OnePanelClient, OnePanelConfig, OnePanelError};
pub use monitoring::MonitoringModule;
pub use websites::WebsitesModule;
pub use apps::AppsModule;
pub use containers::ContainersModule;
pub use logs::LogsModule;
pub use cronjobs::CronjobsModule;
pub use task_center::TaskCenterModule;
pub use nodes::NodesModule;

/// High-level facade for 1Panel operations.
///
/// # Example
///
/// ```rust,no_run
/// use synapse::tools::onepanel::OnePanel;
///
/// let panel = OnePanel::from_env()?;
///
/// let os_info = panel.monitoring().get_os_info().await?;
/// let websites = panel.websites().search(1, 20).await?;
/// let containers = panel.containers().list().await?;
/// ```
pub struct OnePanel {
    client: OnePanelClient,
}

impl OnePanel {
    pub fn new(config: OnePanelConfig) -> Result<Self, OnePanelError> {
        let client = OnePanelClient::new(config)?;
        Ok(Self { client })
    }

    pub fn from_env() -> Result<Self, OnePanelError> {
        let client = OnePanelClient::from_env()?;
        Ok(Self { client })
    }

    pub fn monitoring(&self) -> MonitoringModule<'_> {
        MonitoringModule::new(&self.client)
    }

    pub fn websites(&self) -> WebsitesModule<'_> {
        WebsitesModule::new(&self.client)
    }

    pub fn apps(&self) -> AppsModule<'_> {
        AppsModule::new(&self.client)
    }

    pub fn containers(&self) -> ContainersModule<'_> {
        ContainersModule::new(&self.client)
    }

    pub fn logs(&self) -> LogsModule<'_> {
        LogsModule::new(&self.client)
    }

    pub fn cronjobs(&self) -> CronjobsModule<'_> {
        CronjobsModule::new(&self.client)
    }

    pub fn task_center(&self) -> TaskCenterModule<'_> {
        TaskCenterModule::new(&self.client)
    }

    pub fn nodes(&self) -> NodesModule<'_> {
        NodesModule::new(&self.client)
    }
}
