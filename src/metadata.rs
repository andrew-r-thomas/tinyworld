use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

use serde::{de::DeserializeOwned, Serialize};

pub struct MetaDataThing {
    file: File,
}

impl MetaDataThing {
    pub fn new(path: &Path) -> Result<Self, std::io::Error> {
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        Ok(Self { file })
    }

    pub fn insert(&mut self, meta: &impl Serialize) {
        let buff = rmp_serde::to_vec(meta).unwrap();
        self.file.write(&buff).unwrap();
        self.file.rewind().unwrap();
    }

    pub fn read<M: DeserializeOwned>(&mut self) -> M {
        let mut buff = vec![];
        self.file.read_to_end(&mut buff).unwrap();
        self.file.rewind().unwrap();
        rmp_serde::from_slice(&mut buff).unwrap()
    }

    pub fn clear(&mut self) {
        self.file.set_len(0).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct SimpleMessage {
        text: String,
    }
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct MetaMessage {
        id: usize,
        is_cool: bool,
        text: String,
    }
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    enum EnumMessage {
        Simple(SimpleMessage),
        Meta(MetaMessage),
    }

    #[test]
    fn meta_read_write_simple() {
        let mut thing = MetaDataThing::new(Path::new("test.tw")).unwrap();
        let meta = SimpleMessage {
            text: "this is some text".into(),
        };

        thing.insert(&meta);
        let read_meta: SimpleMessage = thing.read();
        assert_eq!(read_meta, meta);
    }
    #[test]
    fn meta_read_write_enum() {
        let mut thing = MetaDataThing::new(Path::new("test.tw")).unwrap();
        let simple = EnumMessage::Simple(SimpleMessage {
            text: "this is some text".into(),
        });
        let meta = EnumMessage::Meta(MetaMessage {
            id: 0,
            is_cool: true,
            text: "this is some other text".into(),
        });

        thing.insert(&meta);
        let read_meta: EnumMessage = thing.read();
        assert_eq!(read_meta, meta);

        thing.clear();

        thing.insert(&simple);
        let read_simple: EnumMessage = thing.read();
        assert_eq!(read_simple, simple);
    }
}
