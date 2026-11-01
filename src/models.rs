use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinates {
    #[serde(default)]
    pub latitude: f64,
    #[serde(default)]
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    #[serde(rename = "connectionName")]
    pub connection_name: String,
    #[serde(default)]
    pub country: String,
    #[serde(rename = "countryCode", default)]
    pub country_code: String,
    #[serde(default)]
    pub location: String,
    pub load: Option<u32>,
    #[serde(rename = "pubKey")]
    pub pub_key: Option<String>,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
    pub coordinates: Option<Coordinates>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoInfo {
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub country: String,
    #[serde(rename = "countryCode", default)]
    pub country_code: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub secured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    #[serde(rename = "connectionName")]
    pub connection_name: String,
    #[serde(default)]
    pub country: String,
    #[serde(rename = "countryCode", default)]
    pub country_code: String,
    #[serde(default)]
    pub location: String,
    #[serde(rename = "usedAt", default)]
    pub used_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationItem {
    #[serde(rename = "connectionName")]
    pub connection_name: String,
    pub country: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
    pub location: String,
    pub load: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetStatus {
    pub daemon: bool,
    pub configured: bool,
    pub connected: bool,
    pub endpoint: Option<String>,
    pub location: String,
    pub country: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
    pub load: Option<u32>,
    pub ip: String,
    pub secured: bool,
    #[serde(rename = "myCountry")]
    pub my_country: String,
    #[serde(rename = "myCountryCode")]
    pub my_country_code: String,
    #[serde(rename = "myCity")]
    pub my_city: String,
    pub recent: Vec<RecentEntry>,
}
