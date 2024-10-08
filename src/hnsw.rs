use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap, HashMap, HashSet},
    usize,
};

use rand::{rngs::ThreadRng, Rng};
use uuid::Uuid;

use crate::{
    distance_calculators::DistanceCalculator,
    utils::{MaxDist, MinDist},
};

#[derive(Debug)]
pub enum Error {
    EmbSizeError,
}

pub struct FixedParams {
    pub dimension: u32,
    pub m: u32,
    pub m_max: u32,
    pub m0_max: u32,
    pub ef_construction: u32,
    pub level_norm: f32,
    // dist_id: u8,
}

// TODO:
impl Default for FixedParams {
    fn default() -> Self {
        Self {
            dimension: 0,
            m: 24,
            m_max: 50,
            m0_max: 100,
            ef_construction: 50,
            level_norm: 10.0,
        }
    }
}

pub struct HNSW {
    entry: Option<(Uuid, u32)>,
    rng: ThreadRng,
    fixed_params: FixedParams,
    dist_calc: RefCell<Box<dyn DistanceCalculator>>,
    embeddings: HashMap<Uuid, Vec<f32>>,
    connections: Vec<HashMap<Uuid, Vec<(Uuid, f32)>>>,
}

enum Query<'q> {
    Id(Uuid),
    Data(&'q [f32]),
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

    // TODO: figure out best place for ef
    pub fn search(&self, query: &[f32], top_k: usize, ef: usize) -> Vec<(Uuid, f32)> {
        match self.entry {
            Some((mut entry_id, entry_level)) => {
                for level in (1..=entry_level).rev() {
                    entry_id = self.search_layer(Query::Data(query), entry_id, 1, level)[0].0;
                }

                let mut out = self.search_layer(Query::Data(query), entry_id, ef, 0);
                out.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                out.get(0..top_k).unwrap().to_vec()
            }
            None => vec![],
        }
    }

    pub fn insert(&mut self, new_data: &[f32]) -> Result<Uuid, Error> {
        if new_data.len() != self.fixed_params.dimension as usize {
            return Err(Error::EmbSizeError);
        }

        let new_level =
            f32::floor(-f32::ln(self.rng.gen_range(0.0..=1.0)) * self.fixed_params.level_norm)
                as u32;
        println!("new level: {new_level}");
        let new_emb_id = Uuid::new_v4();
        let new_emb_data = Vec::from(new_data);
        self.embeddings.insert(new_emb_id, new_emb_data);

        match &mut self.entry {
            None => {
                self.entry = Some((new_emb_id, new_level));
                for _ in 0..=new_level {
                    self.connections
                        .push(HashMap::from_iter([(new_emb_id, vec![])]));
                }
            }
            Some(ep) => {
                if new_level > ep.1 {
                    for _ in ep.1 as usize..=new_level as usize {
                        self.connections
                            .push(HashMap::from_iter([(new_emb_id, vec![])]));
                    }
                    *ep = (new_emb_id, new_level);
                }

                let entry_point = self.entry.unwrap().clone();
                let mut entry_point_id = entry_point.0;

                // find entry for new level
                for level in (new_level + 1..=entry_point.1).rev() {
                    entry_point_id =
                        self.search_layer(Query::Id(new_emb_id), entry_point_id, 1, level)[0].0;
                }

                // insert node at each level for the rest of the way down
                for level in (0..=new_level).rev() {
                    let top_ef_construction = self.search_layer(
                        Query::Id(new_emb_id),
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

                if new_level > self.entry.unwrap().1 {
                    self.entry = Some((new_emb_id, new_level));
                }
            }
        }

        Ok(new_emb_id)
    }

    fn search_layer(
        &self,
        query: Query,
        entry: Uuid,
        top_k: usize,
        level: u32,
    ) -> Vec<(Uuid, f32)> {
        println!("started search layer");
        let mut candidates = BinaryHeap::<MinDist>::new();
        let mut found = BinaryHeap::<MaxDist>::new();
        let mut visited = HashSet::new();

        let mut dist_calculator = self.dist_calc.borrow_mut();

        let query_data = match query {
            Query::Id(id) => self.embeddings.get(&id).unwrap(),
            Query::Data(data) => data,
        };

        {
            let entry_data = self.embeddings.get(&entry).unwrap();
            let dist = dist_calculator.calc_dist(&query_data, &entry_data);
            visited.insert(entry);
            candidates.push(MinDist(Reverse(MaxDist { id: entry, dist })));
            found.push(MaxDist { dist, id: entry });
        }

        while candidates.len() > 0 {
            let c = candidates.pop().unwrap();
            let f = found.peek().unwrap();

            if c.0 .0.dist > f.dist {
                break;
            }

            let c_nbs = match self
                .connections
                .get(level as usize)
                .unwrap()
                .get(&c.0 .0.id)
            {
                Some(s) => s,
                None => break,
            };

            for nb in c_nbs {
                if !visited.contains(&nb.0) {
                    let dist = {
                        let nb_data = self.embeddings.get(&nb.0).unwrap();
                        dist_calculator.calc_dist(&query_data, &nb_data)
                    };
                    visited.insert(nb.0);

                    let f = found.peek().unwrap();

                    if &dist < &f.dist || found.len() < top_k {
                        candidates.push(MinDist(Reverse(MaxDist { dist, id: nb.0 })));
                        found.push(MaxDist { dist, id: nb.0 });

                        if found.len() > top_k {
                            found.pop();
                        }
                    }
                }
            }
        }

        found.into_iter().map(|f| (f.id, f.dist)).collect()
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
        println!("started select neighbors");
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
            if out.is_empty()
                || &nearest_cand.1
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
