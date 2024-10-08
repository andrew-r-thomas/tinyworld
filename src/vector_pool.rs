use std::collections::{HashMap, HashSet};

use zerocopy::{IntoBytes, TryFromBytes};

use crate::storage_manager::{ItemId, StorageManager};

pub enum VectorPoolError {
    InvalidItemId,
}

pub struct VectorPool {
    pool: Vec<u8>,
    page_to_frame_map: HashMap<u32, usize>,
    empty_frames: Vec<usize>,
    dirty_pages: HashSet<usize>,
    free_slots: Vec<ItemId>,
    page_size: usize,
    vec_size: usize,
    slots_per_page: usize,
}

impl VectorPool {
    pub fn new(pool_size: usize, page_size: usize, vec_size: usize, slots_per_page: usize) -> Self {
        let pool = vec![0; page_size * pool_size];
        let empty_frames = Vec::from_iter(0..pool_size);
        let dirty_pages = HashSet::new();
        let free_slots = Vec::new();
        let page_to_frame_map = HashMap::new();

        Self {
            pool,
            empty_frames,
            dirty_pages,
            free_slots,
            page_size,
            vec_size,
            page_to_frame_map,
            slots_per_page,
        }
    }

    pub fn get(&mut self, id: ItemId, sm: &mut StorageManager) -> Result<&[f32], VectorPoolError> {
        if id.slot_number as usize >= self.slots_per_page {
            return Err(VectorPoolError::InvalidItemId);
        }

        let page_number = id.page_number;
        match self.page_to_frame_map.get(&page_number) {
            Some(frame_number) => {
                let page_start = frame_number * self.page_size;
                let page_end = page_start + self.page_size;
                let page = self.pool.get(page_start..page_end).unwrap();

                let slot_start = id.slot_number as usize;
                let slot_end = slot_start + 1;
                let slot = u8::try_ref_from_bytes(page.get(slot_start..slot_end).unwrap()).unwrap();

                match slot {
                    0 => Err(VectorPoolError::InvalidItemId),
                    1 => {
                        let vec_start = self.slots_per_page as usize
                            + (id.slot_number as usize * self.vec_size);
                        let vec_end = vec_start + self.vec_size;

                        Ok(
                            <[f32]>::try_ref_from_bytes(page.get(vec_start..vec_end).unwrap())
                                .unwrap(),
                        )
                    }
                    _ => panic!(),
                }
            }
            None => match self.empty_frames.pop() {
                Some(idx) => {
                    let frame_start = self.page_size * idx;
                    let frame_end = frame_start + self.page_size;
                    let frame = self.pool.get_mut(frame_start..frame_end).unwrap();

                    sm.read_page(id.page_number, frame);

                    self.page_to_frame_map.insert(id.page_number, idx);
                    let slots = frame.get(0..self.slots_per_page).unwrap();
                    for (slot, slot_number) in slots.iter().zip(0..) {
                        if *slot == 0 {
                            self.free_slots.push(ItemId {
                                page_number,
                                slot_number,
                            });
                        }
                    }

                    let slot_start = id.slot_number as usize;
                    let slot_end = slot_start + 1;
                    let slot =
                        u8::try_ref_from_bytes(frame.get(slot_start..slot_end).unwrap()).unwrap();

                    match slot {
                        0 => Err(VectorPoolError::InvalidItemId),
                        1 => {
                            let vec_start = self.slots_per_page as usize
                                + (id.slot_number as usize * self.vec_size);
                            let vec_end = vec_start + self.vec_size;

                            Ok(
                                <[f32]>::try_ref_from_bytes(frame.get(vec_start..vec_end).unwrap())
                                    .unwrap(),
                            )
                        }
                        _ => panic!(),
                    }
                }
                None => todo!("implement algo for choosing frame to ditch"),
            },
        }
    }

    pub fn push(
        &mut self,
        new: &[f32],
        sm: &mut StorageManager,
    ) -> Result<ItemId, VectorPoolError> {
        // so first we check if any of our in memory pages has a free slot,
        // then we check if we have any free frames, if we dont, we ditch one,
        // if we write a new frame, either from ditching or not, we need to mark
        // the page as dirty, and add a page in the storage manager when we write

        match self.free_slots.pop() {
            Some(item_id) => {
                let page_number = item_id.page_number;
                let slot_number = item_id.slot_number;

                let frame_idx = self.page_to_frame_map.get(&page_number).unwrap();
                let frame_start = frame_idx * self.page_size;
                let frame_end = frame_start + self.page_size;

                let frame = self.pool.get_mut(frame_start..frame_end).unwrap();

                let slot = u8::try_ref_from_bytes(
                    frame
                        .get(slot_number as usize..slot_number as usize + 1)
                        .unwrap(),
                )
                .unwrap();
                match slot {
                    0 => {
                        let vec_start =
                            self.slots_per_page + (slot_number as usize * self.vec_size);
                        let vec_end = vec_start + self.vec_size;
                        let vec = frame.get_mut(vec_start..vec_end).unwrap();

                        vec.copy_from_slice(new.as_bytes());
                        self.dirty_pages.insert(*frame_idx);
                    }
                    _ => panic!(),
                }

                Ok(item_id)
            }
            None => match self.empty_frames.pop() {
                Some(frame_idx) => {
                    let frame_start = frame_idx * self.page_size;
                    let frame_end = frame_start + self.page_size;

                    let frame = self.pool.get_mut(frame_start..frame_end).unwrap();
                    frame[0] = 1;

                    let vec_start = self.slots_per_page;
                    let vec_end = vec_start + self.vec_size;
                    let vec = frame.get_mut(vec_start..vec_end).unwrap();

                    vec.copy_from_slice(new.as_bytes());

                    let page_number = sm.new_page();
                    self.page_to_frame_map.insert(page_number, frame_idx);
                    for slot_number in 1..self.slots_per_page {
                        self.free_slots.push(ItemId {
                            slot_number: slot_number as u32,
                            page_number,
                        })
                    }

                    Ok(ItemId {
                        slot_number: 0,
                        page_number,
                    })
                }
                None => todo!("ditch a slot and do the above in the free space"),
            },
        }
    }

    pub fn flush(&self, sm: &mut StorageManager) {
        todo!("write everything out to disk that needs to")
    }

    fn choose_to_ditch(&mut self) -> usize {
        // lets do approx lru k for now, then see about tuning
        todo!()
    }
}
