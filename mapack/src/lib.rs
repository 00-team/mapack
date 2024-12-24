pub trait Point {
    fn geometry(&self) -> u8;
}

pub trait Tile {
    fn layers(&self) -> &[(&'static str, Vec<Box<dyn Point>>)];
}
