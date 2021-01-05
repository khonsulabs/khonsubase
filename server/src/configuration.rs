use fluent_templates::once_cell::sync::OnceCell;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

mod session_maximum_days;
mod site_name;
mod site_primary_locale;

pub use self::{
    session_maximum_days::SessionMaximumDays, site_name::SiteName,
    site_primary_locale::SitePrimaryLocale,
};

pub trait Configuration {
    type Type: Serialize + DeserializeOwned;

    fn default() -> Option<Self::Type>;
    fn key() -> &'static str;
}

static SHARED_MANAGER: OnceCell<ConfigurationManager> = OnceCell::new();

#[derive(Clone, Debug)]
pub struct ConfigurationManager {
    active_configuration: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl ConfigurationManager {
    pub fn shared() -> Self {
        SHARED_MANAGER
            .get_or_init(|| {
                let active_configuration = Arc::new(RwLock::new(HashMap::new()));

                Self {
                    active_configuration,
                }
            })
            .clone()
    }

    pub fn get<T: Configuration>(&self) -> Option<T::Type> {
        let configuration = self.active_configuration.read().ok()?;
        configuration
            .get(T::key())
            .and_then(|v| serde_json::value::from_value(v.clone()).ok())
            .unwrap_or_else(T::default)
    }
}
