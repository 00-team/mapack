#[cfg(test)]
mod tests {
    mod my_tiles {
        mapack::mapack! {
            poi: {
                name: String,
            },
        }

        impl PointPoi {
            fn decode_name(v: &mapack::Value) -> Option<String> {
                Some(v.string_value().to_string())
            }
        }
    }

    #[test]
    fn test() {
        // my_tiles::Tile::decode(zom, tx, ty, pbf)
    }
}

