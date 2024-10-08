use std::collections::HashMap;

use crate::storage_manager::{self, ItemId, StorageManager};

enum IndexError {
    InvalidLevel,
    InvalidItemId,
}

pub struct Index {
    levels: Vec<HashMap<ItemId, Vec<Conn>>>,
    storage_manager: StorageManager,
}

impl Index {
    pub fn new(storage_manager: StorageManager) -> Self {
        Self {
            levels: vec![],
            storage_manager,
        }
    }

    pub fn get_conns(&self, node: ItemId, level: usize) -> Result<&[Conn], IndexError> {
        match self.levels.get(level) {
            Some(conn_map) => match conn_map.get(&node) {
                Some(conns) => Ok(conns),
                None => Err(IndexError::InvalidItemId),
            },
            None => Err(IndexError::InvalidLevel),
        }
    }

    pub fn push_conn(
        &mut self,
        a: ItemId,
        b: ItemId,
        dist: f32,
        level: usize,
    ) -> Result<(), IndexError> {
        match self.levels.get_mut(level) {
            Some(conn_map) => {
                match conn_map.get_mut(&a) {
                    Some(conns) => {
                        conns.push(Conn { other: b, dist });
                    }
                    None => return Err(IndexError::InvalidItemId),
                }
                match conn_map.get_mut(&b) {
                    Some(conns) => conns.push(Conn { other: a, dist }),
                    None => return Err(IndexError::InvalidItemId),
                }
                Ok(())
            }

            None => Err(IndexError::InvalidLevel),
        }
    }

    pub fn push_item(&mut self, new: ItemId, highest_level: usize) {
        for level in 0..=highest_level {
            match self.levels.get_mut(level) {
                Some(conn_map) => {
                    conn_map.insert(new, vec![]);
                }
                None => {
                    let mut conn_map = HashMap::new();
                    conn_map.insert(new, vec![]);
                    self.levels.push(conn_map);
                }
            }
        }
    }
}

struct Conn {
    other: ItemId,
    dist: f32,
}
