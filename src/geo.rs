use crate::models::GeoInfo;
use std::time::Duration;

pub fn fetch_geo() -> GeoInfo {
    let agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(2))
        .timeout_write(Duration::from_secs(2))
        .timeout_connect(Duration::from_secs(2))
        .build();

    let resp = match agent
        .get("https://api.surfshark.com/v1/server/user")
        .set("User-Agent", "daailouivan-surfshark/1.0")
        .call()
    {
        Ok(r) => r,
        Err(_) => return GeoInfo::default(),
    };

    resp.into_json::<GeoInfo>().unwrap_or_default()
}
