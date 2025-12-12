use std::cell::RefCell;
use std::io::{Read, Result, Seek, SeekFrom};
use std::rc::Rc;

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ?Sized> ReadSeek for T {}

// Type alias for convenience
type SharedReader<'a> = Rc<RefCell<dyn ReadSeek + 'a>>;

#[derive(Clone)]
pub struct SharedReadSeek<'a> {
    inner: SharedReader<'a>,
}

impl<'a> SharedReadSeek<'a> {
    fn new(inner: SharedReader<'a>) -> Self {
        Self { inner }
    }

    pub fn from_read_seek<R: ReadSeek + 'a>(reader: R) -> Self {
        // Wrap the reader in a RefCell and Rc to allow shared ownership and mutable access
        let reader = Rc::new(RefCell::new(reader));
        Self::new(reader)
    }
}

impl Read for SharedReadSeek<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.borrow_mut().read(buf)
    }
}

impl Seek for SharedReadSeek<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.borrow_mut().seek(pos)
    }
}
