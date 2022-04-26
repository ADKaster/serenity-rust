extern crate libc;

pub use ipc_proc::ipc_file;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

// Note: Keep in sync with string_hash().
const fn hash(s: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    let mut i = 0;
    loop {
        hash = hash.wrapping_add(s[i] as u32);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
        i += 1;
        if i == s.len() {
            break;
        }
    }
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    hash
}

pub const fn compute_magic(name: &str) -> u32 {
    hash(name.as_bytes())
}

#[derive(Debug)]
pub enum Dictionary {
    Data(HashMap<String, String>),
}

#[derive(Debug)]
pub struct File {
    fd: i32,
}

fn receive_fd(fd: i32) -> std::io::Result<i32> {
    let received_fd = unsafe { libc::recvfd(fd, libc::O_CLOEXEC) };
    if received_fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(received_fd)
    }
}

fn send_fd(fd: i32, to_fd: i32) -> std::io::Result<()> {
    let rc = unsafe { libc::sendfd(to_fd, fd) };
    if rc < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct UnixSocketStreamBuffer {
    fd: i32,
    data: VecDeque<u8>,
}

#[derive(Debug)]
pub struct UnixSocketStream {
    buffer: UnixSocketStreamBuffer,
    received_fds: FDsToSend,
}

impl UnixSocketStream {
    pub fn new(fd: i32) -> UnixSocketStream {
        UnixSocketStream {
            buffer: UnixSocketStreamBuffer {
                fd: fd,
                data: VecDeque::new(),
            },
            received_fds: FDsToSend::receive(fd),
        }
    }
    pub fn with_buffer(fd: i32, buffer: VecDeque<u8>) -> UnixSocketStream {
        UnixSocketStream {
            buffer: UnixSocketStreamBuffer {
                fd: fd,
                data: buffer,
            },
            received_fds: FDsToSend::receive(fd),
        }
    }

    pub fn fds(&mut self) -> &mut FDsToSend {
        if self.received_fds.fds.is_empty() {
            self.received_fds.receive_fds(self.buffer.fd);
        }

        &mut self.received_fds
    }
}

impl Iterator for UnixSocketStream {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.buffer.next()
    }
}

impl Iterator for UnixSocketStreamBuffer {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.data.is_empty() {
            return None;
        }

        self.data.pop_front()
    }
}

pub struct Message {}

#[derive(Debug)]
pub struct FDsToSend {
    pub fds: Vec<i32>,
}

impl FDsToSend {
    pub fn take(&mut self) -> Option<i32> {
        self.fds.pop()
    }

    pub fn from_vec(fds: Vec<i32>) -> FDsToSend {
        FDsToSend { fds }
    }
    pub fn from_fd(fd: i32) -> FDsToSend {
        FDsToSend { fds: vec![fd] }
    }
    pub fn receive_fds(&mut self, fd: i32) {
        loop {
            match receive_fd(fd) {
                Ok(received_fd) => {
                    self.fds.push(received_fd);
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
    pub fn send_fds(&self, fd: i32) -> std::io::Result<()> {
        for fd_to_send in self.fds.iter() {
            send_fd(*fd_to_send, fd)?;
        }
        Ok(())
    }
    pub fn receive(fd: i32) -> FDsToSend {
        let mut fds = FDsToSend { fds: vec![] };
        fds.receive_fds(fd);
        fds
    }
}

pub trait Encoder<Types> {
    fn encode(data: &Types) -> Option<(Vec<u8>, FDsToSend)>;
    fn encode_value(data: &Types) -> Option<Vec<u8>> {
        if let Some((bytes, fds)) = Self::encode(data) {
            assert!(fds.fds.len() == 0);
            Some(bytes)
        } else {
            None
        }
    }
}

pub trait Decoder<Types> {
    fn decode(it: &mut dyn Iterator<Item = u8>, fds: &mut FDsToSend) -> Option<Types>;
    fn decode_stream(it: &mut UnixSocketStream) -> Option<Types> {
        Self::decode(&mut it.buffer, &mut it.received_fds)
    }
    fn decode_value(it: &mut dyn Iterator<Item = u8>) -> Option<Types> {
        let mut fds = FDsToSend { fds: vec![] };
        Self::decode(it, &mut fds)
    }
}

impl Encoder<()> for Message {
    fn encode(_data: &()) -> Option<(Vec<u8>, FDsToSend)> {
        Some((vec![], FDsToSend { fds: vec![] }))
    }
}

impl Decoder<()> for Message {
    fn decode(_it: &mut dyn Iterator<Item = u8>, _fds: &mut FDsToSend) -> Option<()> {
        Some(())
    }
}

macro_rules! _encode {
    ($result:expr, $fds:expr, $data:expr, $t:ident, $i:tt, $($rest:ident, $rest_i:tt),*) => {
        if let Some(x) = Message::encode(&(($data).$i)) {
            $result.extend(x.0);
            $fds.fds.extend(x.1.fds);
            _encode!($result, $fds, $data, $($rest, $rest_i),*)
        } else {
            None
        }
    };
    ($result:expr, $fds:expr, $data:expr, $t:ident, $i:tt) => {
        if let Some(x) = Message::encode(&(($data).$i)) {
            $result.extend(x.0);
            $fds.fds.extend(x.1.fds);
            Some(($result, $fds))
        } else {
            None
        }
    };
}
macro_rules! impl_encoder {
    ($($($t:ident, $i:tt),+)?) => {
        $(
            impl<$($t,)*> Encoder<($($t,)*)> for Message where Message: $(Encoder<$t>+)* {
                fn encode(data: &($($t,)+)) -> Option<(Vec<u8>, FDsToSend)> {
                    let mut result = vec![];
                    let mut fds = FDsToSend { fds: vec![] };
                    _encode!(result, fds, data, $($t, $i),+)
                }
            }
        )*
    }
}

macro_rules! _decode {
    ($it:expr, $fds:expr, $t:ident) => {
        Message::decode($it, $fds)?
    };
}
macro_rules! impl_decoder {
    ($($($t:ident),+)?) => {
        $(
            impl<$($t,)*> Decoder<($($t,)*)> for Message where Message: $(Decoder<$t>+)* {
                fn decode(it: &mut dyn Iterator<Item=u8>, fds: &mut FDsToSend) -> Option<($($t,)+)> {
                    Some(($(_decode!(it, fds, $t),)+))
                }
            }
        )*
    }
}

impl_encoder!(T0, 0);
impl_encoder!(T0, 0, T1, 1);
impl_encoder!(T0, 0, T1, 1, T2, 2);
impl_encoder!(T0, 0, T1, 1, T2, 2, T3, 3);

impl_decoder!(T0);
impl_decoder!(T0, T1);
impl_decoder!(T0, T1, T2);
impl_decoder!(T0, T1, T2, T3);

impl Encoder<u32> for Message {
    fn encode(data: &u32) -> Option<(Vec<u8>, FDsToSend)> {
        let mut result = vec![];
        result.extend(data.to_le_bytes().iter().cloned());
        Some((result, FDsToSend { fds: vec![] }))
    }
}

impl Encoder<u64> for Message {
    fn encode(data: &u64) -> Option<(Vec<u8>, FDsToSend)> {
        let mut result = vec![];
        result.extend(data.to_le_bytes().iter().cloned());
        Some((result, FDsToSend { fds: vec![] }))
    }
}

impl Decoder<u32> for Message {
    fn decode(it: &mut dyn Iterator<Item = u8>, _fds: &mut FDsToSend) -> Option<u32> {
        let mut result: u32 = 0;
        let mut i = 0;
        for b in it {
            result |= (b as u32) << (i * 8);
            if i == 3 {
                break;
            }
            i += 1;
        }
        if i == 3 {
            Some(result.to_le())
        } else {
            None
        }
    }
}

impl Decoder<u64> for Message {
    fn decode(it: &mut dyn Iterator<Item = u8>, _fds: &mut FDsToSend) -> Option<u64> {
        let mut result: u64 = 0;
        let mut i = 0;
        for b in it {
            result |= (b as u64) << (i * 8);
            if i == 7 {
                break;
            }
            i += 1;
        }
        if i == 7 {
            Some(result.to_le())
        } else {
            None
        }
    }
}

impl Decoder<bool> for Message {
    fn decode(it: &mut dyn Iterator<Item = u8>, _fds: &mut FDsToSend) -> Option<bool> {
        Some(it.next()? != 0)
    }
}

impl Encoder<bool> for Message {
    fn encode(data: &bool) -> Option<(Vec<u8>, FDsToSend)> {
        Some((vec![if *data { 1 } else { 0 }], FDsToSend { fds: vec![] }))
    }
}

impl<T> Decoder<Vec<T>> for Message
where
    Message: Decoder<T>,
{
    fn decode(it: &mut dyn Iterator<Item = u8>, fds: &mut FDsToSend) -> Option<Vec<T>> {
        let mut result = vec![];
        let count: u32 = <Message as Decoder<u32>>::decode(it, fds)?;
        for _ in 0..count {
            result.push(Message::decode(it, fds)?);
        }
        Some(result)
    }
}

impl<T> Encoder<Vec<T>> for Message
where
    Message: Encoder<T>,
{
    fn encode(data: &Vec<T>) -> Option<(Vec<u8>, FDsToSend)> {
        let mut result = vec![];
        let mut fds = FDsToSend { fds: vec![] };
        let x = <Message as Encoder<u32>>::encode(&(data.len() as u32))?;
        result.extend(x.0);
        for d in data {
            let x = Message::encode(d)?;
            result.extend(x.0);
            fds.fds.extend(x.1.fds);
        }
        Some((result, fds))
    }
}

impl Encoder<Dictionary> for Message {
    fn encode(dictionary: &Dictionary) -> Option<(Vec<u8>, FDsToSend)> {
        // Note: Same as HashMap<String, String>, but the size is a u64.
        match dictionary {
            Dictionary::Data(data) => {
                let mut result = vec![];
                let mut fds = FDsToSend { fds: vec![] };
                let x = <Message as Encoder<u64>>::encode(&(data.len() as u64))?;
                result.extend(x.0);
                for (k, v) in data {
                    let x = Message::encode(k)?;
                    result.extend(x.0);
                    fds.fds.extend(x.1.fds);
                    let x = Message::encode(v)?;
                    result.extend(x.0);
                    fds.fds.extend(x.1.fds);
                }
                Some((result, fds))
            }
        }
    }
}

impl Decoder<Dictionary> for Message {
    fn decode(it: &mut dyn Iterator<Item = u8>, fds: &mut FDsToSend) -> Option<Dictionary> {
        // Note: Same as HashMap<String, String>, but the size is a u64.
        let mut result = HashMap::new();
        let count: u64 = Message::decode(it, fds)?;
        for _ in 0..count {
            let key: String = Message::decode(it, fds)?;
            let value: String = Message::decode(it, fds)?;
            result.insert(key, value);
        }
        Some(Dictionary::Data(result))
    }
}

impl<K, V> Encoder<HashMap<K, V>> for Message
where
    Message: Encoder<K> + Encoder<V>,
{
    fn encode(data: &HashMap<K, V>) -> Option<(Vec<u8>, FDsToSend)> {
        let mut result = vec![];
        let mut fds = FDsToSend { fds: vec![] };
        let x = <Message as Encoder<u32>>::encode(&(data.len() as u32))?;
        result.extend(x.0);
        for (k, v) in data {
            let x = Message::encode(k)?;
            result.extend(x.0);
            fds.fds.extend(x.1.fds);
            let x = Message::encode(v)?;
            result.extend(x.0);
            fds.fds.extend(x.1.fds);
        }
        Some((result, fds))
    }
}

impl<K, V> Decoder<HashMap<K, V>> for Message
where
    Message: Decoder<K> + Decoder<V>,
    K: Eq + Hash,
{
    fn decode(it: &mut dyn Iterator<Item = u8>, fds: &mut FDsToSend) -> Option<HashMap<K, V>> {
        let mut result = HashMap::new();
        let count: u32 = <Message as Decoder<u32>>::decode(it, fds)?;
        for _ in 0..count {
            let k = Message::decode(it, fds)?;
            let v = Message::decode(it, fds)?;
            result.insert(k, v);
        }
        Some(result)
    }
}

impl Encoder<String> for Message {
    fn encode(data: &String) -> Option<(Vec<u8>, FDsToSend)> {
        let mut result = vec![];
        result.extend(<Message as Encoder<u32>>::encode(&(data.len() as u32))?.0);
        if data.len() > 0 {
            result.extend(data.as_bytes());
        }
        Some((result, FDsToSend { fds: vec![] }))
    }
}

impl Decoder<String> for Message {
    fn decode(it: &mut dyn Iterator<Item = u8>, fds: &mut FDsToSend) -> Option<String> {
        let len: u32 = <Message as Decoder<u32>>::decode(it, fds)?;
        if len == 0xfffffff {
            return Some(String::new());
        }

        let data = it.take(len as usize).collect::<Vec<u8>>();
        if data.len() != len as usize {
            return None;
        }

        if let Ok(string) = String::from_utf8(data) {
            Some(string)
        } else {
            None
        }
    }
}

impl Encoder<File> for Message {
    fn encode(data: &File) -> Option<(Vec<u8>, FDsToSend)> {
        Some((vec![], FDsToSend::from_fd(data.fd)))
    }
}

impl Decoder<File> for Message {
    fn decode(_it: &mut dyn Iterator<Item = u8>, fds: &mut FDsToSend) -> Option<File> {
        let fd = fds.take()?;
        Some(File { fd })
    }
}

#[macro_export]
macro_rules! ipc_generate_method_ids {
    (request, $i:expr, $name:ident, |, $($rest:ident $(, $pipe:tt)?),*) => {
        pub const $name: u32 = $i;
        ipc_generate_method_ids!(request, $i + 1, $($rest$(, $pipe)?),*);
    };
    (request, $i:expr, $name:ident, $($rest:ident $(, $pipe:tt)?),*) => {
        pub const $name: u32 = $i;
        ipc_generate_method_ids!(request, $i + 2, $($rest$(, $pipe)?),*);
    };
    (request, $i:expr, $name:ident, |) => {
        pub const $name: u32 = $i;
    };
    (request, $i:expr, $name:ident) => {
        pub const $name: u32 = $i;
    };
    (response, $i:expr, $name:ident, |, $($rest:ident $(, $pipe:tt)?),*) => {
        ipc_generate_method_ids!(response, $i + 2, $($rest$(, $pipe)?),*);
    };
    (response, $i:expr, $name:ident, $($rest:ident $(, $pipe:tt)?),*) => {
        pub const $name: u32 = $i + 1;
        ipc_generate_method_ids!(response, $i + 1, $($rest$(, $pipe)?),*);
    };
    (response, $i:expr, $name:ident, |) => {
    };
    (response, $i:expr, $name:ident) => {
        pub const $name: u32 = $i + 1;
    };
}

#[macro_export]
macro_rules! ipc {
    (
        $(#$_:ident < $($_1:ident $(.$_2:ident)*)/+ >)*
        endpoint $name:ident
        {
            $(
                $method:ident($($([$($_arg_attr:ident$(=$_arg_attr_value:expr)?),+])? $arg_ty:ident$(::$arg_path_ty:ident)*$(<$($arg_ty_param:ty),+>)? $arg:ident),*)
                $(
                     => ($($([$($_ret_attr:ident$(=$_ret_attr_value:expr)?),+])? $ret_ty:ident$(::$ret_path_ty:ident)*$(<$($ret_ty_param:ty),+>)? $ret:ident),*)
                )?
                $(= $pipe:tt)?
            )*
        }
    ) => {
        mod $name {
            use super::*;

            pub mod ResponseId {
                ipc_generate_method_ids!(response, 1, $($method $(, $pipe)?),*);
            }
            pub mod RequestId {
                ipc_generate_method_ids!(request, 1, $($method $(, $pipe)?),*);
            }
            #[derive(Debug)]
            pub enum Response {
                $($method(($($($ret_ty$(::$ret_path_ty)*$(<$($ret_ty_param),*>)?),*),*)),)*
            }
            #[derive(Debug)]
            pub enum Request {
                $($method(($($arg_ty$(::$arg_path_ty)*$(<$($arg_ty_param),*>)?),*)),)*
            }
            #[derive(Debug)]
            pub enum Message {
                Request(Request),
                Response(Response),
            }
            pub static magic: u32 = $crate::ipc::compute_magic(stringify!($name));
        }
    }
}
