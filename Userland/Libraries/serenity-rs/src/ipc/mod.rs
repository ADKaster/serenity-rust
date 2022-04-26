extern crate libc;

use crate::core;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub struct Encoder<'a> {
    output: &'a mut Vec<u8>,
    socket_fd: i32,
}

impl<'a> Encoder<'a> {
    pub fn new(output: &'a mut Vec<u8>, magic: u32, socket_fd: i32) -> Encoder {
        let mut encoder = Encoder { output, socket_fd };
        encoder.encode_u32(magic);
        encoder
    }

    pub fn encode_bool(&mut self, value: bool) -> Option<()> {
        self.output.try_reserve(1).ok()?;
        self.output.push(value as u8);
        Some(())
    }

    pub fn encode_u32(&mut self, value: u32) -> Option<()> {
        self.output.try_reserve(4).ok()?;
        self.output.push(value as u8);
        self.output.push((value >> 8) as u8);
        self.output.push((value >> 16) as u8);
        self.output.push((value >> 24) as u8);
        Some(())
    }

    pub fn encode_u64(&mut self, value: u64) -> Option<()> {
        self.output.try_reserve(8).ok()?;
        self.output.push(value as u8);
        self.output.push((value >> 8) as u8);
        self.output.push((value >> 16) as u8);
        self.output.push((value >> 24) as u8);
        self.output.push((value >> 32) as u8);
        self.output.push((value >> 40) as u8);
        self.output.push((value >> 48) as u8);
        self.output.push((value >> 56) as u8);
        Some(())
    }

    pub fn encode_string(&mut self, string: &String) -> Option<()> {
        self.encode_u32(string.len() as u32)?;
        self.output.try_reserve(string.len()).ok()?;
        for b in string.bytes() {
            self.output.push(b);
        }
        Some(())
    }

    pub fn encode_anonymous_buffer(&mut self, buffer: &core::AnonymousBuffer) -> Option<()> {
        self.encode_bool(buffer.is_valid());
        if buffer.is_valid() {
            self.encode_u32(buffer.size() as u32)?;
            if unsafe { libc::sendfd(self.socket_fd, buffer.fd()) } < 0 {
                return None;
            }
        }
        Some(())
    }

    pub fn encode_dictionary(&mut self, dictionary: &HashMap<String, String>) -> Option<()> {
        self.encode_u64(dictionary.len() as u64);
        for (name, value) in dictionary {
            self.encode_string(name)?;
            self.encode_string(value)?;
        }
        Some(())
    }
}

fn as_u32_le(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 0) +
        ((array[1] as u32) << 8) +
        ((array[2] as u32) << 16) +
        ((array[3] as u32) << 24)
}

fn as_u64_le(array: &[u8; 8]) -> u64 {
    ((array[0] as u64) << 0) +
        ((array[1] as u64) << 8) +
        ((array[2] as u64) << 16) +
        ((array[3] as u64) << 24) +
        ((array[4] as u64) << 32) +
        ((array[5] as u64) << 40) +
        ((array[6] as u64) << 48) +
        ((array[7] as u64) << 56)
}

fn receive_fd(fd: i32) -> std::io::Result<i32> {
    let received_fd = unsafe { libc::recvfd(fd, libc::O_CLOEXEC) };
    if received_fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(received_fd)
    }
}

pub struct Decoder<'a> {
    bytes: &'a mut VecDeque<u8>,
    socket_fd: i32,
}

impl<'a> Decoder<'a> {
    pub fn with_bytes(bytes: &mut VecDeque<u8>, socket_fd: i32) -> Decoder {
        Decoder { bytes, socket_fd }
    }

    pub fn decode_u32(&mut self) -> Option<u32> {
        let mut values = [0u8; 4];
        for i in 0..4 {
            values[i] = self.bytes.pop_front()?;
        }
        Some(as_u32_le(&values))
    }

    pub fn decode_u64(&mut self) -> Option<u64> {
        let mut values = [0u8; 8];
        for i in 0..8 {
            values[i] = self.bytes.pop_front()?;
        }
        Some(as_u64_le(&values))
    }

    pub fn decode_bool(&mut self) -> Option<bool> {
        let value = self.bytes.pop_front()?;
        Some(value != 0)
    }

    pub fn decode_anonymous_buffer(&mut self) -> Option<Arc<core::AnonymousBuffer>> {
        let valid = self.decode_bool()?;
        if !valid {
            return Some(core::AnonymousBuffer::new());
        }
        let size = self.decode_u32()?;
        let fd = receive_fd(self.socket_fd).ok()?;
        match core::AnonymousBuffer::from_fd(fd as i32, size as usize) {
            Err(_) => None,
            Ok(buffer) => Some(buffer),
        }
    }

    pub fn decode_string(&mut self) -> Option<String> {
        let length = self.decode_u32()?;
        if length == 0xffffffff {
            // NOTE: We can't represent Serenity's null AK::String with a Rust String.
            //       But we'd like to move away from those anyway, so let's just use an empty String.
            Some(String::new())
        } else {
            let mut raw = Vec::new();
            raw.try_reserve(length as usize).ok()?;
            for _ in 0..length {
                raw.push(self.bytes.pop_front()?);
            }
            String::from_utf8(raw).ok()
        }
    }

    pub fn decode_dictionary(&mut self) -> Option<HashMap<String, String>> {
        let length = self.decode_u64()?;

        let mut dictionary = HashMap::<String, String>::new();
        dictionary.try_reserve(length as usize).ok()?;

        for _ in 0..length {
            let key = self.decode_string()?;
            let value = self.decode_string()?;
            dictionary.insert(key, value);
        }

        Some(dictionary)
    }
}
