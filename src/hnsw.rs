use std::collections::HashMap;

use rand::{rngs::ThreadRng, Rng};
use uuid::Uuid;

pub enum Error {
    EmbSizeError,
}

pub struct FixedParams {
    dimension: u32,
    m: u32,
    m_max: u32,
    m0_max: u32,
    ef_construction: u32,
    level_norm: f32,
    dist_id: u8,
}

pub struct HNSW {
    entry: Option<Embedding>,
    rng: ThreadRng,
    fixed_params: FixedParams,
    dist_calc: Box<dyn DistanceCalculator>,
}

impl HNSW {
    pub fn new(fixed_params: FixedParams, dist_calc: Box<dyn DistanceCalculator>) -> Self {
        let rng = rand::thread_rng();
        Self {
            entry: None,
            rng,
            fixed_params,
            dist_calc,
        }
    }

    /*
    so the basic idea here is:
    1. get a level for the node,
    2. traverse the layers to find the best entry point for the new node at its highest level
    3. at each level going down from the desired level of the new node:
        - add M (param) connections out of the top efConstruction (param) nodes to the nearest nodes from the entry point on the level,
        - for each of the nodes new neighbors on the level, prune them if they have more than Mmax (or Mmax0 for the bottom) connections,
        - continue traversing to the next layer
    if the new level is higher than the max, we need to update stuff
    */
    pub fn insert(&mut self, new_data: &[f32]) -> Result<(), Error> {
        if new_data.len() != self.fixed_params.dimension as usize {
            return Err(Error::EmbSizeError);
        }

        let new_level =
            f32::floor(-f32::ln(self.rng.gen_range(0.0..=1.0)) * self.fixed_params.level_norm)
                as usize;
        // TODO: gotta add this to embeddings
        let new_emb = Embedding::new(new_data, new_level);

        if let None = self.entry {
            self.entry = Some(new_emb);
        } else {
            let mut entry_point_id = self.entry.as_ref().unwrap().id.clone();
            let entry_point_level = self.entry.as_ref().unwrap().level;
            // find entry for new level
            for level in (new_level + 1..=entry_point_level).rev() {
                entry_point_id = *self
                    .search_layer(&new_emb.id, &entry_point_id, 1, level)
                    .next()
                    .unwrap();
            }
            // insert node at each level for the rest of the way down
            for level in (0..=new_level).rev() {
                let m_max = match level {
                    0 => self.fixed_params.m0_max as usize,
                    _ => self.fixed_params.m_max as usize,
                };

                // NOTE: top_m should be sorted in order of ascending distance
                let top_ef_construction = self.search_layer(
                    &new_emb.id,
                    &entry_point_id,
                    self.fixed_params.ef_construction,
                    level,
                );
                let selected_neighbors = self.select_neighbors(
                    &new_emb.id,
                    top_ef_construction,
                    self.fixed_params.m as usize,
                    level,
                );
                for neighbor in selected_neighbors {
                    self.add_connection(neighbor, &new_emb.id);
                    if self.get_num_connections(neighbor) > m_max {
                        let new_neighbors = self.select_neighbors(
                            neighbor,
                            self.get_neighbors(neighbor),
                            m_max,
                            level,
                        );

                        self.set_neighbors(neighbor, new_neighbors);
                    }
                }
            }
        }

        Ok(())
    }

    fn search_layer(
        &self,
        _query: &Uuid,
        _entry: &Uuid,
        _top_k: u32,
        _level: usize,
    ) -> impl Iterator<Item = &Uuid> {
        // TODO:
        [].iter()
    }

    fn select_neighbors<'select_neighbors>(
        &self,
        _query: &Uuid,
        _candidates: impl Iterator<Item = &'select_neighbors Uuid>,
        _top_k: usize,
        _level: usize,
    ) -> impl Iterator<Item = &Uuid> {
        // TODO:
        [].iter()
    }

    fn add_connection(&mut self, _a: &Uuid, _b: &Uuid) {
        todo!()
    }

    fn get_num_connections(&self, _emb_id: &Uuid) -> usize {
        todo!()
    }

    fn get_neighbors(&self, _emb_id: &Uuid) -> impl Iterator<Item = &Uuid> {
        // TODO:
        [].iter()
    }

    fn set_neighbors<'set_neighbors>(
        &self,
        _emb_id: &Uuid,
        _new_neighbors: impl Iterator<Item = &'set_neighbors Uuid>,
    ) {
        todo!()
    }
}

pub struct Embedding {
    id: Uuid,
    data: Vec<f32>,
    level: usize,
}

impl Embedding {
    pub fn new(data: &[f32], top_level: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            data: Vec::from(data),
            level: top_level,
        }
    }
}

pub trait DistanceCalculator {}
