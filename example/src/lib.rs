#[cfg(test)]
mod tests {
    use mapack::Coordinate;

    mod my_tiles {
        mapack::mapack! {
            poi: {
                name: String,
            },
        }
    }

    #[test]
    fn test() {
        let latitude = 35.55293745336477;
        let longitude = 50.38793775563117;

        let mut my_tile = my_tiles::Tile::new();
        let coords = Coordinate::from_latlng(16, latitude, longitude);

        my_tile.poi.push(my_tiles::PointPoi {
            id: Some(12),
            name: "new poi".to_string(),
            coordinate: coords.clone(),
        });

        let pbf = my_tile.encode().expect("encode error");

        let old_tile = my_tiles::Tile::decode(
            coords.zoom(),
            coords.tx(),
            coords.ty(),
            pbf.clone(),
        )
        .expect("decode error");

        let op = &old_tile.poi[0];
        let mp = &my_tile.poi[0];

        assert_eq!(mp.name, op.name);
        assert_eq!(mp.id, op.id);
        assert!(mp.coordinate.distance_to(&op.coordinate) < 1.0, "distance");

        assert_eq!(old_tile.encode().unwrap(), pbf);
    }
}
