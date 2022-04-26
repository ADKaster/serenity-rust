#![feature(rustc_private)]
#![allow(dead_code)]
#![allow(unused_imports)]

extern crate libc;

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::io::ErrorKind::UnexpectedEof;
use std::io::{BufReader, Read, Result, Write};
use std::mem::transmute;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{mem, os, thread};

use serenity::core::AnonymousBuffer;
use serenity::{dbgln, ipc};

#[derive(Debug)]
enum ServerMessage {
    Disconnected,
    GetClipboardData,
    SetClipboardData {
        data: Arc<AnonymousBuffer>,
        mime_type: String,
        metadata: HashMap<String, String>,
    },
    GetClipboardDataResponse {
        data: Arc<AnonymousBuffer>,
        mime_type: String,
        metadata: HashMap<String, String>,
    },
}

enum ClientMessage {
    ClipboardDataChanged { mime_type: String },
}

struct ConnectionFromClient {
    stream: UnixStream,
    messages: VecDeque<ServerMessage>,
    bytes: VecDeque<u8>,
}

struct GlobalSharedState {
    data: Arc<AnonymousBuffer>,
    mime_type: String,
    metadata: HashMap<String, String>,
}

pub fn hexdump(bytes: &[u8], nread: usize) {
    let mut str = String::new();
    for i in 0..nread {
        str.push_str(format!("{:02x} ", bytes[i]).as_str());
        if i > 0 && i % 16 == 0 {
            str.push('\n');
        }
    }
    dbgln!("{}", str);
}

fn as_u32_le(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 0)
        + ((array[1] as u32) << 8)
        + ((array[2] as u32) << 16)
        + ((array[3] as u32) << 24)
}

fn read_u32(bytes: &mut VecDeque<u8>) -> Option<u32> {
    let mut values = [0u8; 4];
    for i in 0..4 {
        values[i] = bytes.pop_front()?;
    }
    Some(as_u32_le(&values))
}

impl ConnectionFromClient {
    pub fn fd(&self) -> i32 { self.stream.as_raw_fd() }

    pub fn new(stream: UnixStream) -> ConnectionFromClient {
        ConnectionFromClient {
            stream,
            messages: VecDeque::new(),
            bytes: VecDeque::new(),
        }
    }

    fn decode_message(&self, mut bytes: VecDeque<u8>) -> Option<ServerMessage> {
        let mut decoder = ipc::Decoder::with_bytes(&mut bytes, self.stream.as_raw_fd());

        let magic = decoder.decode_u32()?;

        if magic != 1329211611 {
            dbgln!("Bad magic! {} instead of 1329211611", magic);
            return None;
        }

        let message_id = decoder.decode_u32()?;
        match message_id {
            1 => {
                // GetClipboardData
                Some(ServerMessage::GetClipboardData)
            }
            2 => {
                // GetClipboardDataResponse
                todo!(":yakthonk:")
            }
            3 => {
                // SetClipboardData
                let data = decoder.decode_anonymous_buffer()?;
                let mime_type = decoder.decode_string()?;
                let metadata = decoder.decode_dictionary()?;
                Some(ServerMessage::SetClipboardData {
                    data,
                    mime_type,
                    metadata,
                })
            }
            _ => None,
        }
    }

    fn populate_message_queue(&mut self) -> std::io::Result<()> {
        let mut buffer = [0u8; 4096];
        let nread = self.stream.read(&mut buffer)?;
        if nread == 0 {
            return Ok(());
        }
        dbgln!("Read {} bytes from client:", nread);
        hexdump(&buffer, nread);

        for i in 0..nread {
            self.bytes.push_back(buffer[i]);
        }

        loop {
            let length = read_u32(&mut self.bytes);
            if length.is_none() {
                return Ok(());
            }
            let length = length.unwrap();

            let mut msg = VecDeque::<u8>::new();
            for _i in 0..length {
                msg.push_back(self.bytes.pop_front().unwrap());
            }

            let message = self.decode_message(msg);
            if message.is_none() {
                return Ok(());
            }

            self.messages.push_back(message.unwrap());
        }
    }

    pub fn wait_for_message(&mut self) -> std::io::Result<ServerMessage> {
        self.populate_message_queue()?;
        if self.messages.is_empty() {
            return Ok(ServerMessage::Disconnected);
        }
        Ok(self.messages.pop_front().unwrap())
    }

    pub fn send_message(&mut self, message: ServerMessage) -> std::io::Result<()> {
        let mut buffer = Vec::<u8>::new();
        let mut encoder = ipc::Encoder::new(&mut buffer, 1329211611, self.stream.as_raw_fd());
        match &message {
            ServerMessage::GetClipboardDataResponse {
                data,
                metadata,
                mime_type,
            } => {
                encoder.encode_u32(2); // MessageID::GetClipboardDataResponse
                encoder.encode_anonymous_buffer(data);
                encoder.encode_string(mime_type);
                encoder.encode_dictionary(metadata);
            }
            _ => {
                todo!("Encoding of {:?}", message);
            }
        }

        dbgln!("Encoded {:?} as follows:", message);
        hexdump(buffer.as_slice(), buffer.len());

        let len = buffer.len() as u32;
        self.stream.write(&len.to_le_bytes())?;
        self.stream.write(buffer.as_slice())?;
        Ok(())
    }
}

fn handle_client(
    stream: std::os::unix::net::UnixStream,
    global_shared_state: &mut Arc<Mutex<GlobalSharedState>>,
) -> std::io::Result<()> {
    let mut connection_from_client = ConnectionFromClient::new(stream);

    while let Ok(message) = connection_from_client.wait_for_message() {
        match message {
            ServerMessage::GetClipboardData => {
                dbgln!("GetClipboardData");
                let global_shared_state = global_shared_state.lock().unwrap();
                connection_from_client.send_message(ServerMessage::GetClipboardDataResponse {
                    data: global_shared_state.data.clone(),
                    mime_type: global_shared_state.mime_type.clone(),
                    metadata: global_shared_state.metadata.clone(),
                })?;
            }
            ServerMessage::SetClipboardData {
                data,
                mime_type,
                metadata,
            } => {
                dbgln!("SetClipboardData");
                let mut global_shared_state = global_shared_state.lock().unwrap();
                global_shared_state.data = data;
                global_shared_state.mime_type = mime_type;
                global_shared_state.metadata = metadata;
            }
            ServerMessage::Disconnected => {
                dbgln!("Disconnected");
                break;
            }
            _ => {
                dbgln!("Unknown message: {:?}", message);
            }
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut fd = -1;

    if let Ok(socket_takeover) = std::env::var("SOCKET_TAKEOVER") {
        if let Some((_path, number)) = socket_takeover.split_once(':') {
            fd = number.parse().unwrap();
            // NOTE: We have to make the socket non-blocking or UnixListener will fall apart on the first EAGAIN.
            let value = 0;
            unsafe {
                libc::ioctl(fd, libc::FIONBIO, &value);
            }
        } else {
            return Ok(());
        }
    }
    if fd < 0 {
        dbgln!("Unable to take over socket from system server!");
        return Ok(());
    }
    let listener = unsafe { UnixListener::from_raw_fd(fd) };

    let global_shared_state = Arc::new(Mutex::new(GlobalSharedState {
        data: AnonymousBuffer::new(),
        mime_type: String::from("text/plain"),
        metadata: HashMap::new(),
    }));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                dbgln!("Got connection! {:?}", stream);
                let mut global_shared_state = Arc::clone(&global_shared_state);
                thread::spawn(move || {
                    if let Err(err) = handle_client(stream, &mut global_shared_state) {
                        dbgln!("Client disconnected with error: {}", err);
                    }
                });
            }
            Err(err) => {
                dbgln!("An error occurred: {}", err);
                break;
            }
        }
    }

    Ok(())
}
