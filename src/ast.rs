#[derive(Debug)]
pub struct Pdf {
    pub objects: Vec<Object>,
}

#[derive(Debug)]
pub enum Object {
    Stream {
        id: i32,
        generation: i32,
        body: ByteStream,
    },
    Number(i32),
}

#[derive(Debug)]
pub struct ByteStream {
    pub bytes: Vec<u8>,
}
