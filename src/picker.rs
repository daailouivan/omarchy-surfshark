use crate::models::{Coordinates, GeoInfo, Server};
use rand::seq::SliceRandom;
use std::collections::HashMap;

const USA_DEFAULT: &str = "us-nyc.prod.surfshark.com";

pub fn filter_generic(servers: &[Server]) -> Vec<&Server> {
    servers
        .iter()
        .filter(|s| {
            s.server_type.as_deref().unwrap_or("") == "generic"
                && !s.connection_name.is_empty()
                && s.pub_key.is_some()
        })
        .collect()
}

pub fn haversine_distance(p1: &Coordinates, p2: &Coordinates) -> f64 {
    let r = 6371.0; // Earth radius in km
    let d_lat = (p2.latitude - p1.latitude).to_radians();
    let d_lon = (p2.longitude - p1.longitude).to_radians();

    let lat1 = p1.latitude.to_radians();
    let lat2 = p2.latitude.to_radians();

    let a = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

pub fn approximate_user_position(servers: &[Server], geo: &GeoInfo) -> Option<Coordinates> {
    if geo.country_code.is_empty() {
        return None;
    }
    let pts: Vec<&Coordinates> = servers
        .iter()
        .filter(|s| s.country_code == geo.country_code)
        .filter_map(|s| s.coordinates.as_ref())
        .collect();

    if pts.is_empty() {
        return None;
    }

    let lat_sum: f64 = pts.iter().map(|p| p.latitude).sum();
    let lon_sum: f64 = pts.iter().map(|p| p.longitude).sum();
    let count = pts.len() as f64;

    Some(Coordinates {
        latitude: lat_sum / count,
        longitude: lon_sum / count,
    })
}

pub fn pick_fastest<'a>(
    servers: &'a [Server],
    geo: &GeoInfo,
    suggested: Option<&'a Server>,
) -> Option<&'a Server> {
    if let Some(s) = suggested {
        if s.pub_key.is_some() {
            return Some(s);
        }
    }
    pick_nearest(servers, geo, true, false)
}

pub fn pick_nearest<'a>(
    servers: &'a [Server],
    geo: &GeoInfo,
    by_load: bool,
    foreign: bool,
) -> Option<&'a Server> {
    let mut pool: Vec<&Server> = filter_generic(servers);
    if pool.is_empty() {
        return None;
    }

    if foreign && !geo.country_code.is_empty() {
        let without_home: Vec<&Server> = pool
            .iter()
            .copied()
            .filter(|s| s.country_code != geo.country_code)
            .collect();
        if !without_home.is_empty() {
            pool = without_home;
        }
    }

    let pos = approximate_user_position(servers, geo);
    let mut with_coords: Vec<&Server> = pool
        .iter()
        .copied()
        .filter(|s| s.coordinates.is_some())
        .collect();

    if with_coords.is_empty() {
        return pool.into_iter().min_by_key(|s| s.load.unwrap_or(100));
    }

    if let Some(user_pos) = pos {
        with_coords.sort_by(|a, b| {
            let dist_a = haversine_distance(&user_pos, a.coordinates.as_ref().unwrap());
            let dist_b = haversine_distance(&user_pos, b.coordinates.as_ref().unwrap());
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        if by_load {
            let top_slice = &with_coords[..with_coords.len().min(12)];
            top_slice.iter().copied().min_by_key(|s| s.load.unwrap_or(100))
        } else {
            with_coords.first().copied()
        }
    } else {
        with_coords.into_iter().min_by_key(|s| s.load.unwrap_or(100))
    }
}

pub fn pick_same<'a>(servers: &'a [Server], geo: &GeoInfo) -> Option<&'a Server> {
    if geo.country_code.is_empty() {
        return None;
    }
    let pool: Vec<&Server> = filter_generic(servers)
        .into_iter()
        .filter(|s| s.country_code == geo.country_code)
        .collect();

    pool.into_iter().min_by_key(|s| s.load.unwrap_or(100))
}

pub fn pick_usa<'a>(servers: &'a [Server], preferred: Option<&str>) -> Option<&'a Server> {
    let pool: Vec<&Server> = filter_generic(servers)
        .into_iter()
        .filter(|s| s.country_code == "US")
        .collect();

    if pool.is_empty() {
        return None;
    }

    let want = preferred.unwrap_or(USA_DEFAULT);
    for s in &pool {
        if s.connection_name == want {
            return Some(s);
        }
    }

    pool.into_iter().min_by_key(|s| s.load.unwrap_or(100))
}

pub fn pick_random<'a>(
    servers: &'a [Server],
    geo: &GeoInfo,
    current_host: Option<&str>,
) -> Option<&'a Server> {
    let pool: Vec<&Server> = filter_generic(servers);
    let mut by_country: HashMap<String, Vec<&Server>> = HashMap::new();

    for s in pool {
        if s.country_code.is_empty() || s.country_code == geo.country_code {
            continue;
        }
        by_country.entry(s.country_code.clone()).or_default().push(s);
    }

    if by_country.is_empty() {
        return None;
    }

    if let Some(ch) = current_host {
        if let Some(cur) = servers.iter().find(|s| s.connection_name == ch) {
            if by_country.len() > 1 {
                by_country.remove(&cur.country_code);
            }
        }
    }

    let mut rng = rand::thread_rng();
    let countries: Vec<&String> = by_country.keys().collect();
    let chosen_country = countries.choose(&mut rng)?;

    let candidates = by_country.get(*chosen_country)?;
    candidates.iter().copied().min_by_key(|s| s.load.unwrap_or(100))
}
