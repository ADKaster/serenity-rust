extern crate libc;

use crate::ipc;
use std::ffi::c_void;
use std::ptr::slice_from_raw_parts_mut;
use std::sync::Arc;

#[derive(Debug)]
pub struct AnonymousBuffer {
    fd: i32,
    size: usize,
    data: *mut c_void,
}

unsafe impl Send for AnonymousBuffer {}
unsafe impl Sync for AnonymousBuffer {}

impl AnonymousBuffer {
    pub fn new() -> Arc<AnonymousBuffer> {
        Arc::new(AnonymousBuffer {
            fd: -1,
            size: 0,
            data: 0 as *mut c_void,
        })
    }

    pub fn new_with_size(size: usize) -> std::io::Result<Arc<AnonymousBuffer>> {
        let fd = unsafe { libc::anon_create(size, libc::O_CLOEXEC) };
        if fd < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            AnonymousBuffer::from_fd(fd, size)
        }
    }

    pub fn from_fd(fd: i32, size: usize) -> std::io::Result<Arc<AnonymousBuffer>> {
        let data = unsafe {
            libc::mmap(
                0 as *mut c_void,
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if data == libc::MAP_FAILED {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Arc::new(AnonymousBuffer { fd, size, data }))
        }
    }

    pub fn is_valid(&self) -> bool {
        self.fd != -1
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn fd(&self) -> i32 {
        self.fd
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        let ptr = slice_from_raw_parts_mut(self.data, self.size) as *mut [u8];
        unsafe { &mut *ptr }
    }
}

impl ipc::Encoder<Arc<AnonymousBuffer>> for ipc::Message {
    fn encode(data: &Arc<AnonymousBuffer>) -> Option<(Vec<u8>, ipc::FDsToSend)> {
        let (mut bytes, mut fds) = ipc::Message::encode(&data.is_valid())?;
        if !data.is_valid() {
            return Some((bytes, fds));
        }

        if let Some((b, f)) = ipc::Message::encode(&(data.size() as u32)) {
            bytes.extend(b);
            fds.fds.extend(f.fds);
        } else {
            return None;
        }
        fds.fds.push(data.fd());
        Some((bytes, fds))
    }
}

impl ipc::Decoder<Arc<AnonymousBuffer>> for ipc::Message {
    fn decode(
        data: &mut dyn Iterator<Item = u8>,
        fds: &mut ipc::FDsToSend,
    ) -> Option<Arc<AnonymousBuffer>> {
        let is_valid = ipc::Message::decode(data, fds)?;
        if is_valid {
            let size: u32 = ipc::Message::decode(data, fds)?;
            let fd = fds.take()?;
            Some(AnonymousBuffer::from_fd(fd, size as usize).ok()?)
        } else {
            Some(AnonymousBuffer::new())
        }
    }
}
