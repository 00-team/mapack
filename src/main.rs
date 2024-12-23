use std::collections::HashMap;

use mapack::{Point, Tile};

struct EateryPoint {
    gene: String,
    is_private: bool,
    is_for_guest: bool,
    category: u8,
    name: String,
}

impl Point for EateryPoint {
    fn geometry(&self) -> u8 {
        10
    }
}

struct IssuePoint {
    gene: String,
    kind: u8,
}

impl Point for IssuePoint {
    fn geometry(&self) -> u8 {
        19
    }
}

struct MyTile {
    eatery: Vec<EateryPoint>,
    issue: Vec<IssuePoint>,
}

impl Tile for MyTile {
    fn layers(&self) -> &[(&'static str, Vec<Box<dyn Point>>)] {
        &[("eatery", self.eatery), ("issue", self.issue)]
    }
}

fn main() {}
