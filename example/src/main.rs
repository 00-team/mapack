mod tiles {
    mapack::mapack! {
        eatery: {
            // __max_zoom_level: Config::MAX_ZOOM,
            gene: String,
            is_private: bool,
            is_for_guest: bool,
            category: u8,
            name: String,
        },
        issue: {
            gene: String,
            kind: u8,
        },
    }
}

fn main() {
    println!("hi from example")
}
