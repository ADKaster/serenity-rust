#![feature(rustc_private)]
#![allow(dead_code)]
#![allow(unused_imports)]

extern crate libc;

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::io::ErrorKind::UnexpectedEof;
use std::io::{BufReader, Error, ErrorKind, Read, Result, Write};
use std::mem::transmute;
use std::ops::Deref;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{mem, os, thread};

#[macro_use(ipc, ipc_generate_method_ids)]
extern crate serenity;

use serenity::core::AnonymousBuffer;
use serenity::dbgln;
use serenity::ipc::*;
use serenity::{ipc, Core, IPC};

ipc_file!("../Clipboard/ClipboardServer.ipc");

struct ConnectionFromClient {
    stream: UnixStream,
    messages: VecDeque<ClipboardServer::Message>,
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

impl ConnectionFromClient {
    pub fn fd(&self) -> i32 {
        self.stream.as_raw_fd()
    }

    pub fn new(stream: UnixStream) -> ConnectionFromClient {
        ConnectionFromClient {
            stream,
            messages: VecDeque::new(),
            bytes: VecDeque::new(),
        }
    }

    fn decode_message(&self, bytes: VecDeque<u8>) -> Option<ClipboardServer::Message> {
        use serenity::ipc::Message as Decoder;

        let mut stream = serenity::ipc::UnixSocketStream::with_buffer(self.fd(), bytes);
        let magic: u32 = Decoder::decode_value(&mut stream)?;

        if magic != ClipboardServer::magic {
            dbgln!("Bad magic! {} instead of {}", magic, ClipboardServer::magic);
            return None;
        }

        let message_id: u32 = Decoder::decode_value(&mut stream)?;
        match message_id {
            ClipboardServer::RequestId::get_clipboard_data => {
                Some(ClipboardServer::Request::get_clipboard_data(()))
            }
            ClipboardServer::RequestId::set_clipboard_data => Some(
                ClipboardServer::Request::set_clipboard_data(Decoder::decode_stream(&mut stream)?),
            ),
            _ => {
                dbgln!("decode_message: unknown message id {}", message_id);
                None
            }
        }
        .map(|request| ClipboardServer::Message::Request(request))
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

        let mut it = self.bytes.iter().cloned();
        loop {
            let length = serenity::ipc::Message::decode_value(&mut it);
            if length.is_none() {
                return Ok(());
            }
            let length: u32 = length.unwrap();

            let mut msg = VecDeque::<u8>::new();
            for _i in 0..length {
                msg.push_back(
                    it.next()
                        .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "EOF"))?,
                );
            }

            let message = self.decode_message(msg);
            if message.is_none() {
                return Ok(());
            }

            self.messages.push_back(message.unwrap());
        }
    }

    pub fn wait_for_message(&mut self) -> std::io::Result<ClipboardServer::Message> {
        self.populate_message_queue()?;
        if self.messages.is_empty() {
            Err(std::io::Error::from(ErrorKind::NotConnected))
        } else {
            Ok(self.messages.pop_front().unwrap())
        }
    }

    pub fn send_message(&mut self, message: ClipboardServer::Message) -> std::io::Result<()> {
        let mut buffer = Vec::<u8>::new();
        let mut fds = ipc::FDsToSend::from_vec(Vec::new());
        use serenity::ipc::Message as Encoder;
        buffer.extend(
            Encoder::encode_value(&ClipboardServer::magic)
                .ok_or(Error::from(ErrorKind::InvalidData))?,
        );

        match &message {
            ClipboardServer::Message::Response(ClipboardServer::Response::get_clipboard_data(
                data,
            )) => {
                buffer.extend(
                    Encoder::encode_value(&ClipboardServer::ResponseId::get_clipboard_data)
                        .ok_or(Error::from(ErrorKind::InvalidData))?,
                );
                let (data, f) = Encoder::encode(data).ok_or(Error::from(ErrorKind::InvalidData))?;
                buffer.extend(data);
                fds.fds.extend(f.fds);
            }
            _ => {
                todo!("Encoding of {:?}", message);
            }
        }

        dbgln!("Encoded {:?} as follows:", message);
        let len = buffer.len() as u32;
        let len_bytes = len.to_le_bytes();
        hexdump(len_bytes.as_slice(), len_bytes.len());
        hexdump(buffer.as_slice(), buffer.len());

        self.stream.write(&len_bytes)?;
        self.stream.write(buffer.as_slice())?;
        fds.send_fds(self.stream.as_raw_fd())?;
        Ok(())
    }
}

fn handle_client(
    stream: std::os::unix::net::UnixStream,
    global_shared_state: &mut Arc<Mutex<GlobalSharedState>>,
) -> std::io::Result<()> {
    let mut connection_from_client = ConnectionFromClient::new(stream);

    while let Ok(message) = connection_from_client.wait_for_message() {
        dbgln!("Got message: {:?}", message);
        match message {
            ClipboardServer::Message::Request(ClipboardServer::Request::get_clipboard_data(())) => {
                let global_shared_state = global_shared_state.lock().unwrap();
                connection_from_client.send_message(ClipboardServer::Message::Response(
                    ClipboardServer::Response::get_clipboard_data((
                        global_shared_state.data.clone(),
                        global_shared_state.mime_type.clone(),
                        ipc::Dictionary::Data(global_shared_state.metadata.clone()),
                    )),
                ))?;
            }
            ClipboardServer::Message::Request(ClipboardServer::Request::set_clipboard_data((
                data,
                mime_type,
                metadata,
            ))) => {
                let mut global_shared_state = global_shared_state.lock().unwrap();
                global_shared_state.data = data;
                global_shared_state.mime_type = mime_type;
                match metadata {
                    ipc::Dictionary::Data(metadata) => {
                        global_shared_state.metadata = metadata;
                    }
                }
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
