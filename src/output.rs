//! An in-memory output stream that can be shared with a [`crate::session::Session`].

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

/// A cloneable byte buffer implementing [`Write`]; clones share contents.
/// Useful for capturing what a program prints.
#[derive(Debug, Clone, Default)]
pub struct SharedBuffer(Rc<RefCell<Vec<u8>>>);

impl SharedBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything written so far, as text.
    pub fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.borrow()).into_owned()
    }

    /// Returns and clears the contents.
    pub fn take(&self) -> String {
        let bytes = std::mem::take(&mut *self.0.borrow_mut());
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
