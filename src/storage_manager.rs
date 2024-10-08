use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, Write},
    path::Path,
};

use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, Unaligned};

pub struct StorageManager {
    file: File,
    // TODO: maybe have the header/page directory pulled in here so that we can
    // check against an immutable version of it for things being changed on the fly,
    // this doesnt make sense but yeah think about it
    page_size: u8, // kb
    // TODO: this is dirty
    num_pages: u32,
}

#[repr(packed)]
#[derive(
    // bruh
    TryFromBytes,
    Immutable,
    KnownLayout,
    Unaligned,
    IntoBytes,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
)]
pub struct ItemId {
    pub slot_number: u32,
    pub page_number: u32,
}

const HEADER_SIZE: usize = 32;
#[repr(packed)]
#[derive(TryFromBytes, Immutable, KnownLayout, Unaligned, IntoBytes)]
pub struct Header {
    // for the storage manager
    page_size: u8, // in kb
    num_pages: u32,

    // for the vector pool
    vec_page_slots: u32,
    dim: u32,

    // for the index, probably wont end up being in the header
    // TODO: also need to add a listing of index pages since there might be multiple
    m_max: u8,
    m0_max: u8,
    m: u8,
    m_l: f32,
    dist_id: u32,
    ep: ItemId,
}

#[derive(Debug)]
pub enum StorageManagerError {
    FileTypeError,
    ZeroCopyError,
    IoError(io::Error),
}

impl StorageManager {
    pub fn open(path: &Path) -> Result<(Self, Header), StorageManagerError> {
        match path.extension() {
            Some(e) => {
                if !(e == "tw") {
                    return Err(StorageManagerError::FileTypeError);
                }
            }
            None => return Err(StorageManagerError::FileTypeError),
        }
        let mut file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(f) => f,
            Err(e) => return Err(StorageManagerError::IoError(e)),
        };

        let mut header_buff = [0; HEADER_SIZE];
        file.read_exact(&mut header_buff).unwrap();
        let header = match Header::try_read_from_bytes(&header_buff) {
            Ok(h) => h,
            Err(_) => return Err(StorageManagerError::ZeroCopyError),
        };

        Ok((
            Self {
                file,
                page_size: header.page_size,
                num_pages: header.num_pages,
            },
            header,
        ))
    }

    pub fn create(
        path: &Path,
        m_max: u8,
        m0_max: u8,
        m: u8,
        dim: u32,
        dist_id: u32,
        m_l: f32,
    ) -> Result<(Self, Header), StorageManagerError> {
        match path.extension() {
            Some(e) => {
                if !(e == "tw") {
                    return Err(StorageManagerError::FileTypeError);
                }
            }
            None => return Err(StorageManagerError::FileTypeError),
        }
        let mut file = match OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(path)
        {
            Ok(f) => f,
            Err(e) => return Err(StorageManagerError::IoError(e)),
        };

        // TODO:
        let page_size = 4;
        let vec_page_slots = 1;
        let header = Header {
            num_pages: 0,
            m_max,
            m0_max,
            m_l,
            m,
            dim,
            dist_id,
            page_size,
            vec_page_slots,
            ep: ItemId {
                page_number: 0,
                slot_number: 0,
            },
        };

        let mut header_buff = [0; HEADER_SIZE];
        header.write_to(&mut header_buff).unwrap();

        file.write_all(&header_buff).unwrap();

        Ok((
            Self {
                file,
                page_size: header.page_size,
                num_pages: header.num_pages,
            },
            header,
        ))
    }

    // yeah just get rid of all these unwraps lol
    pub fn read_page(&mut self, page: u32, buffer: &mut [u8]) {
        let offset = HEADER_SIZE as u32 + (page * self.page_size as u32 * 1000);

        self.file.seek(io::SeekFrom::Start(offset as u64)).unwrap();

        self.file.read_exact(buffer).unwrap();
    }

    // add check for offset being past the eof, and also test that to see what happens
    pub fn write_page(&mut self, page: u32, buffer: &[u8]) {
        let offset = HEADER_SIZE as u32 + (page * self.page_size as u32 * 1000);

        self.file.seek(io::SeekFrom::Start(offset as u64)).unwrap();

        self.file.write_all(buffer).unwrap();
    }

    pub fn new_page(&mut self) -> u32 {
        let page_number = self.num_pages;
        self.num_pages += 1;
        page_number
    }
}
