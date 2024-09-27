use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    usize,
};

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
    entry: Option<(Uuid, u32)>,
    rng: ThreadRng,
    fixed_params: FixedParams,
    dist_calc: RefCell<Box<dyn DistanceCalculator>>,
    embeddings: HashMap<Uuid, Vec<f32>>,
    // NOTE: could maybe use xor to get rid of duplication
    connections: Vec<HashMap<Uuid, Vec<(Uuid, f32)>>>,
}

impl HNSW {
    pub fn new(fixed_params: FixedParams, dist_calc: RefCell<Box<dyn DistanceCalculator>>) -> Self {
        let rng = rand::thread_rng();
        Self {
            entry: None,
            rng,
            fixed_params,
            dist_calc,
            embeddings: HashMap::new(),
            connections: vec![],
        }
    }

    pub fn insert(&mut self, new_data: &[f32]) -> Result<(), Error> {
        if new_data.len() != self.fixed_params.dimension as usize {
            return Err(Error::EmbSizeError);
        }

        // TODO: deal with new level being higher than current entry
        let new_level =
            f32::floor(-f32::ln(self.rng.gen_range(0.0..=1.0)) * self.fixed_params.level_norm)
                as u32;
        let new_emb_id = Uuid::new_v4();
        let new_emb_data = Vec::from(new_data);
        self.embeddings.insert(new_emb_id, new_emb_data);

        if let None = self.entry {
            self.entry = Some((new_emb_id, new_level));
        } else {
            let entry_point = self.entry.unwrap().clone();
            let mut entry_point_id = entry_point.0;

            // find entry for new level
            for level in (new_level + 1..=entry_point.1).rev() {
                entry_point_id = self.search_layer(new_emb_id, entry_point_id, 1, level)[0].0;
            }

            // insert node at each level for the rest of the way down
            for level in (0..=new_level).rev() {
                let top_ef_construction = self.search_layer(
                    new_emb_id,
                    entry_point_id,
                    self.fixed_params.ef_construction as usize,
                    level,
                );
                let selected_neighbors = self.select_neighbors(
                    new_emb_id,
                    top_ef_construction,
                    self.fixed_params.m as usize,
                    level,
                    // TODO:
                    None,
                    None,
                );

                {
                    self.connections
                        .get_mut(level as usize)
                        .unwrap()
                        .insert(new_emb_id, selected_neighbors.clone());
                }

                let m_max = match level {
                    0 => self.fixed_params.m0_max as usize,
                    _ => self.fixed_params.m_max as usize,
                };
                for neighbor in selected_neighbors {
                    {
                        self.connections
                            .get_mut(level as usize)
                            .unwrap()
                            .get_mut(&neighbor.0)
                            .unwrap()
                            .push((new_emb_id, neighbor.1));
                    }
                    let update = {
                        let n_conns = self
                            .connections
                            .get(level as usize)
                            .unwrap()
                            .get(&neighbor.0)
                            .unwrap();
                        match n_conns.len() > m_max {
                            true => {
                                let selected = self.select_neighbors(
                                    neighbor.0,
                                    n_conns.clone(),
                                    m_max,
                                    level,
                                    // TODO:
                                    None,
                                    None,
                                );
                                Some(selected)
                            }
                            false => None,
                        }
                    };

                    if let Some(new) = update {
                        self.connections
                            .get_mut(level as usize)
                            .unwrap()
                            .insert(neighbor.0, new);
                    }
                }
            }
        }

        Ok(())
    }

    fn search_layer(&self, query: Uuid, entry: Uuid, top_k: usize, level: u32) -> Vec<(Uuid, f32)> {
        let mut candidates = HashMap::new();
        let mut found = HashMap::new();
        let mut visited = HashMap::new();

        let mut dist_calculator = self.dist_calc.borrow_mut();

        let query_data = self.embeddings.get(&query).unwrap();

        {
            let entry_data = self.embeddings.get(&entry).unwrap();
            let dist = dist_calculator.calc_dist(&query_data, &entry_data);
            visited.insert(entry, dist);
            candidates.insert(entry, dist);
            found.insert(entry, dist);
        }

        while candidates.len() > 0 {
            let c = candidates
                .iter()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap();
            let f = found
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap();

            if c.1 > f.1 {
                break;
            }

            let c_nbs = self
                .connections
                .get(level as usize)
                .unwrap()
                .get(c.0)
                .unwrap();

            for nb in c_nbs {
                if !visited.contains_key(&nb.0) {
                    let dist = {
                        let nb_data = self.embeddings.get(&nb.0).unwrap();
                        dist_calculator.calc_dist(&query_data, &nb_data)
                    };
                    visited.insert(nb.0, dist);

                    let f = found
                        .iter()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                        .unwrap();

                    if &dist < f.1 || found.len() < top_k {
                        candidates.insert(nb.0, dist);
                        found.insert(nb.0, dist);

                        if found.len() > top_k {
                            let furthest = {
                                found
                                    .iter()
                                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                                    .unwrap()
                                    .0
                                    .clone()
                            };

                            found.remove(&furthest);
                        }
                    }
                }
            }
        }

        found.into_iter().collect()
    }

    fn select_neighbors(
        &self,
        query: Uuid,
        candidates: Vec<(Uuid, f32)>,
        top_k: usize,
        level: u32,
        keep_pruned: Option<bool>,
        extend_cand: Option<bool>,
    ) -> Vec<(Uuid, f32)> {
        let ext = match extend_cand {
            Some(b) => b,
            None => false,
        };
        let kp = match keep_pruned {
            Some(b) => b,
            None => true,
        };

        let mut cand_queue: HashMap<Uuid, f32> = HashMap::from_iter(candidates.clone().into_iter());
        let mut cand_discard: HashMap<Uuid, f32> = HashMap::new();
        let mut out: HashMap<Uuid, f32> = HashMap::new();

        if ext {
            for c in candidates {
                let c_nbs = self
                    .connections
                    .get(level as usize)
                    .unwrap()
                    .get(&c.0)
                    .unwrap();
                for c_nb in c_nbs {
                    if !cand_queue.contains_key(&c_nb.0) {
                        let dist = {
                            let mut dist_calc = self.dist_calc.borrow_mut();
                            let query_data = self.embeddings.get(&query).unwrap();
                            let c_nb_data = self.embeddings.get(&c_nb.0).unwrap();
                            dist_calc.calc_dist(&query_data, &c_nb_data)
                        };
                        cand_queue.insert(c_nb.0, dist);
                    }
                }
            }
        }

        while cand_queue.len() > 0 && out.len() < top_k {
            let nearest_cand_id = {
                cand_queue
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    .unwrap()
                    .0
                    .clone()
            };
            let nearest_cand = cand_queue.remove_entry(&nearest_cand_id).unwrap();
            if &nearest_cand.1
                < out
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    .unwrap()
                    .1
            {
                out.insert(nearest_cand.0, nearest_cand.1);
            } else {
                cand_discard.insert(nearest_cand.0, nearest_cand.1);
            }
        }

        if kp {
            while cand_discard.len() > 0 && out.len() < top_k {
                let nearest_discard_id = {
                    cand_discard
                        .iter()
                        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                        .unwrap()
                        .0
                        .clone()
                };

                let nearest_discard = cand_discard.remove_entry(&nearest_discard_id).unwrap();
                out.insert(nearest_discard.0, nearest_discard.1);
            }
        }

        out.into_iter().collect()
    }
}

pub trait DistanceCalculator {
    fn calc_dist(&mut self, a: &[f32], b: &[f32]) -> f32;
}
