use std::cmp;

use uuid::Uuid;

pub struct MaxDist {
    pub dist: f32,
    pub id: Uuid,
}

impl Ord for MaxDist {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.dist.total_cmp(&other.dist)
    }
}

impl PartialOrd for MaxDist {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.dist.total_cmp(&other.dist))
    }
}

impl Eq for MaxDist {}

impl PartialEq for MaxDist {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct MinDist(pub cmp::Reverse<MaxDist>);
