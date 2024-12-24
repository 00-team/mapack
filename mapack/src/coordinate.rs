use core::f64::consts::PI;

const MAX_ZOOM: u8 = 22;

#[derive(Debug, Clone)]
pub struct Coordinate {
    zom: u8,
    /// latitude
    lat: f64,
    /// longitude
    lng: f64,
    /// screen x
    sx: u32,
    /// screen y
    sy: u32,
    /// tile x
    tx: u32,
    /// tile y
    ty: u32,
}

impl Coordinate {
    pub fn from_latlng(zoom: u8, latitude: f64, longitude: f64) -> Self {
        let zom = zoom.clamp(0, MAX_ZOOM);
        let lat: f64 = latitude.clamp(-90.0, 90.0);
        let lng: f64 = longitude.clamp(-180.0, 180.0);
        let mut coords = Self { zom, lat, lng, sx: 0, sy: 0, tx: 0, ty: 0 };
        coords.update_index();

        coords
    }

    pub fn set_zoom(&mut self, zoom: u8) {
        self.zom = zoom.clamp(0, MAX_ZOOM);
        self.update_index();
    }

    pub fn with_zoom(&self, zoom: u8) -> Self {
        let mut new = self.clone();
        new.set_zoom(zoom);
        new
    }

    fn update_index(&mut self) {
        let n = (1 << self.zom) as f64;
        let tx = (self.lng + 180.0) / 360.0 * n;
        let ty = (1.0 - self.lat.to_radians().tan().asinh() / PI) / 2.0 * n;

        self.sx = ((tx % 1.0) * 4096.0) as u32;
        self.sy = 4096 - ((ty % 1.0) * 4096.0) as u32;

        self.tx = tx as u32;
        self.ty = ty as u32;
    }

    pub fn from_screen(zom: u8, tx: u32, ty: u32, sx: u32, sy: u32) -> Self {
        let zom = zom.clamp(0, MAX_ZOOM);
        let mut coords = Self { zom, lat: 0.0, lng: 0.0, tx, ty, sy, sx };

        let n = (1 << zom) as f64;
        let (tx, ty, sx, sy) = (tx as f64, ty as f64, sx as f64, sy as f64);
        let x = tx + (sx / 4096.0);
        let y = ty + ((4096.0 - sy) / 4096.0);
        let lng = (x / n) * 360.0 - 180.0;
        let lat = (PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees();

        coords.lat = lat;
        coords.lng = lng;

        coords
    }

    pub fn from_geometry(zom: u8, tx: u32, ty: u32, geom: [u32; 3]) -> Self {
        let sx = geom[1] as i32;
        let sy = geom[2] as i32;
        let sx = (sx >> 1) ^ (-(sx & 1));
        let sy = (sy >> 1) ^ (-(sy & 1));
        let sy = 4096 - sy;

        Self::from_screen(zom, tx, ty, sx as u32, sy as u32)
    }

    pub fn to_geometry(&self) -> [u32; 3] {
        let cmd = (1u32 & 0x7) | (1 << 3);
        let x = (self.sx << 1) ^ (self.sx >> 31);
        let y = 4096 - self.sy;
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

    pub fn zoom(&self) -> u8 {
        self.zom
    }

    pub fn latitude(&self) -> f64 {
        self.lat
    }

    pub fn longitude(&self) -> f64 {
        self.lng
    }

    pub fn sx(&self) -> u32 {
        self.sx
    }

    pub fn sy(&self) -> u32 {
        self.sy
    }

    pub fn tx(&self) -> u32 {
        self.tx
    }

    pub fn ty(&self) -> u32 {
        self.ty
    }
}

#[cfg(test)]
mod tests {
    use crate::Coordinate;

    #[test]
    fn coordinate() {
        let data = [
            (12, 35.55280736473704, 50.38718834763207, 2621, 1614, 1205, 1179),
            (
                22,
                35.55280736473704,
                50.38718834763207,
                2684205,
                1653465,
                1201,
                2842,
            ),
        ];

        for (zm, lt, ln, tx, ty, sx, sy) in data {
            let fl = Coordinate::from_latlng(zm, lt, ln);

            assert_eq!(fl.tx, tx, "tx");
            assert_eq!(fl.ty, ty, "ty");
            assert_eq!(fl.sx, sx, "sx");
            assert_eq!(fl.sy, sy, "sy");

            let fs = Coordinate::from_screen(zm, tx, ty, sx, sy);
            assert!((fl.lat - fs.lat).abs() < 0.00001);
            assert!((fl.lng - fs.lng).abs() < 0.00001);

            let gs = Coordinate::from_geometry(zm, tx, ty, fs.to_geometry());
            assert_eq!(gs.sy, fs.sy);
            assert_eq!(gs.sx, fs.sx);

            assert_eq!(gs.lat, fs.lat);
            assert_eq!(gs.lng, fs.lng);
        }
    }
}
