use crate::{config::Config, models::AppErr};
use protobuf::Message;
use sabad::eatery::Eatery;
use shah::{db::entity::Entity, Gene};
use std::f64::consts::PI;
use vector_tile::tile::{Feature, GeomType, Value};

mod vector_tile;
pub mod web;

#[derive(Debug, Default, Clone)]
pub struct Point {
    pub lat: f64,
    pub lng: f64,
    pub zom: u8,
}

impl Point {
    pub fn new(latitude: f64, longitude: f64, zoom: u8) -> Self {
        Self {
            lat: latitude.clamp(-90.0, 90.0),
            lng: longitude.clamp(-180.0, 180.0),
            zom: zoom.clamp(0, Config::MAX_ZOOM),
        }
    }

    pub fn with_zoom(&self, zoom: u8) -> Self {
        Self {
            lat: self.lat,
            lng: self.lng,
            zom: zoom.clamp(0, Config::MAX_ZOOM),
        }
    }

    pub fn from_eatery(eatery: &Eatery) -> Self {
        Self { lat: eatery.latitude, lng: eatery.longitude, zom: eatery.zoom }
    }

    pub fn to_col_row(&self) -> (u32, u32) {
        let lat_rad = self.lat.to_radians();
        let n = (1 << self.zom) as f64;
        let x = (self.lng + 180.0) / 360.0 * n;
        let y = (1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n;

        (x as u32, y as u32)
    }

    pub fn to_screen(&self) -> (u32, u32) {
        let lat_rad = self.lat.to_radians();
        let n = (1 << self.zom) as f64;
        let x = ((((self.lng + 180.0) / 360.0) * n) % 1.0) * 4096.0;
        let y = ((1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n) % 1.0;
        let y = 4096 - ((y * 4096.0) as u32);

        (x as u32, y)
    }

    pub fn from_screen(zom: u8, col: u32, row: u32, x: u32, y: u32) -> Self {
        let zom = zom.clamp(0, Config::MAX_ZOOM);
        let n = (1 << zom) as f64;
        let (col, row, x, y) = (col as f64, row as f64, x as f64, y as f64);
        let col = col + (x / 4096.0);
        let row = row + ((4096.0 - y) / 4096.0);
        let lng = (col / n) * 360.0 - 180.0;
        let lat = (PI * (1.0 - 2.0 * row / n)).sinh().atan().to_degrees();
        Self { zom, lat, lng }
    }

    pub fn from_geometry(zom: u8, col: u32, row: u32, geom: [u32; 3]) -> Self {
        let x = geom[1] as i32;
        let y = geom[2] as i32;
        let x = (x >> 1) ^ (-(x & 1));
        let y = (y >> 1) ^ (-(y & 1));
        let y = 4096 - y;
        Self::from_screen(zom, col, row, x as u32, y as u32)
    }

    pub fn to_geometry(&self) -> [u32; 3] {
        let cmd = (1u32 & 0x7) | (1 << 3);
        let (x, y) = self.to_screen();
        let y = 4096 - y;
        let x = (x << 1) ^ (x >> 31);
        let y = (y << 1) ^ (y >> 31);

        [cmd, x, y]
    }

    /// Implementation of Haversine distance between two points. in meters
    pub fn distance_to(&self, other: &Self) -> f64 {
        let haversine_fn = |theta: f64| (1.0 - theta.cos()) / 2.0;

        let phi1 = self.lat.to_radians();
        let phi2 = other.lat.to_radians();
        let lam1 = self.lng.to_radians();
        let lam2 = other.lng.to_radians();

        let hav_delta_phi = haversine_fn(phi2 - phi1);
        let hav_delta_lam = phi1.cos() * phi2.cos() * haversine_fn(lam2 - lam1);
        let total_delta = hav_delta_phi + hav_delta_lam;

        (2.0 * 6378137.0 * total_delta.sqrt().asin() * 1e3).round() / 1e3
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::Point;

    #[test]
    fn point() {
        let latitude: f64 = 38.22307753495298;
        let longitude: f64 = 44.88368942776003;
        let point = Point::new(latitude, longitude, Config::MAX_ZOOM);
        println!("point: {point:?}");
        let (col, row) = point.to_col_row();
        let (x, y) = point.to_screen();
        println!("col: {col} | row: {row} | x: {x} | y: {y}");
        let new = Point::from_screen(Config::MAX_ZOOM, col, row, x, y);
        println!("new: {new:?}");
        println!("lat: {}", point.lat - new.lat);
        println!("lng: {}", point.lng - new.lng);
    }
}

#[derive(Debug, Default, Clone)]
pub struct Marker {
    point: Point,
    geometry: [u32; 3],
    pub gene: Gene,
    pub is_private: bool,
    pub is_for_guest: bool,
    pub category: u8,
    pub name: String,
}

impl Marker {
    pub fn from_eatery(eatery: &Eatery) -> Self {
        let point = Point::from_eatery(eatery);
        Self {
            geometry: point.to_geometry(),
            point,
            gene: eatery.gene,
            is_private: eatery.is_private(),
            is_for_guest: eatery.is_for_guest(),
            category: eatery.category,
            name: eatery.name().to_string(),
        }
    }

    // pub fn set_point(&mut self, point: Point) {
    //     self.geometry = point.to_geometry();
    //     self.point = point;
    // }

    pub fn set_zoom(&mut self, zoom: u8) {
        self.point.zom = zoom.clamp(0, Config::MAX_ZOOM);
        self.geometry = self.point.to_geometry();
    }

    pub fn point(&self) -> &Point {
        &self.point
    }

    pub fn geometry(&self) -> [u32; 3] {
        self.geometry
    }
}

impl Value {
    pub fn from_string(value: String) -> Self {
        Self { string_value: Some(value), ..Default::default() }
    }

    pub fn from_bool(v: bool) -> Self {
        Self { bool_value: Some(v), ..Default::default() }
    }

    pub fn from_uint(v: u64) -> Self {
        Self { uint_value: Some(v), ..Default::default() }
    }
}

const KEYS: [&str; 5] =
    ["gene", "is_private", "is_for_guest", "category", "name"];

pub fn encode(markers: Vec<Marker>) -> Result<Vec<u8>, AppErr> {
    let mut values = Vec::<Value>::with_capacity(markers.len() * 4);
    let mut features = Vec::<Feature>::with_capacity(markers.len());

    for marker in markers {
        let gene = values.len() as u32;
        values.push(Value::from_string(marker.gene.as_hex()));

        let is_private = values.len() as u32;
        values.push(Value::from_bool(marker.is_private));

        let is_for_guest = values.len() as u32;
        values.push(Value::from_bool(marker.is_for_guest));

        let category = values.len() as u32;
        values.push(Value::from_uint(marker.category as u64));

        let name = values.len() as u32;
        values.push(Value::from_string(marker.name));

        features.push(Feature {
            tags: vec![
                0,
                gene,
                1,
                is_private,
                2,
                is_for_guest,
                3,
                category,
                4,
                name,
            ],
            geometry: marker.geometry.to_vec(),
            type_: Some(protobuf::EnumOrUnknown::new(GeomType::POINT)),
            ..Default::default()
        });
    }

    let layer = vector_tile::tile::Layer {
        name: Some(String::from("eatery")),
        extent: Some(4096),
        version: Some(2),
        features,
        keys: KEYS.map(|k| k.to_string()).to_vec(),
        values,
        ..Default::default()
    };

    let tile = vector_tile::Tile { layers: vec![layer], ..Default::default() };
    let bytes = tile.write_to_bytes()?;

    Ok(bytes)
}

pub fn decode_marker(
    feature: &Feature, values: &[Value],
) -> Result<Marker, &'static str> {
    let tags = &feature.tags;
    if tags.is_empty() {
        return Err("no tags");
    }
    if tags.len() % 2 != 0 {
        return Err("bad tags length");
    }
    if feature.geometry.len() != 3 {
        return Err("bad geometry");
    }

    let mut marker = Marker {
        geometry: feature.geometry.clone().try_into().unwrap(),
        ..Default::default()
    };

    let mut tags_iter = tags.iter();
    loop {
        let Some(k) = tags_iter.next() else { break };
        let Some(v) = tags_iter.next() else { break };
        let k = *k as usize;
        let v = *v as usize;
        if k >= KEYS.len() || v >= values.len() {
            return Err("invalid tags");
        }
        let v = &values[v];

        match KEYS[k] {
            "gene" => {
                let Ok(gene) = v.string_value().parse::<Gene>() else {
                    return Err("invalid marker gene");
                };
                marker.gene = gene;
            }
            "is_private" => marker.is_private = v.bool_value(),
            "is_for_guest" => marker.is_for_guest = v.bool_value(),
            "category" => marker.category = v.uint_value() as u8,
            "name" => marker.name = v.string_value().to_string(),
            _ => return Err("unknown key in marker tags"),
        }
    }

    Ok(marker)
}

pub fn decode(pbf: Vec<u8>) -> Result<Vec<Marker>, AppErr> {
    let tile = vector_tile::Tile::parse_from_bytes(&pbf)?;
    if tile.layers.len() != 1 {
        return Ok(vec![]);
    }
    let layer = &tile.layers[0];
    if layer.version() != 2 || layer.name() != "eatery" {
        return Ok(vec![]);
    }

    let mut markers = Vec::<Marker>::with_capacity(layer.features.len());

    for feature in layer.features.iter() {
        match decode_marker(feature, &layer.values) {
            Ok(v) => markers.push(v),
            Err(e) => {
                log::warn!("found an invalid marker: {e:?}")
            }
        }
    }

    Ok(markers)
}
