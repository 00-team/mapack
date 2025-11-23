pub use mapack_macros::mapack;

mod coordinate;
pub use coordinate::Coordinate;
pub use protobuf;
mod vector_tile;
pub use vector_tile::tile::{Feature, GeomType, Layer, Value};
pub use vector_tile::Tile;

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

    pub fn from_int(v: i64) -> Self {
        Self { int_value: Some(v), ..Default::default() }
    }
}
