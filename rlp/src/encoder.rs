///https://blog.csdn.net/ggq89/article/details/78629008

extern crate bytebuffer;

use self::bytebuffer::*;
use defines::*;
use types::*;
use std::io::{Read, Write, Result};
use std::mem::*;
use std::iter::FromIterator;

macro_rules! total_bytes {
    ($e:expr) => {
        if ($e << 8) == 0 { 1u8 }
        else if ($e << 16) == 0 { 2u8 }
        else if ($e << 24) == 0 { 3u8 }
        else if ($e << 32) == 0 { 4u8 }
        else if ($e << 40) == 0 { 5u8 }
        else if ($e << 48) == 0 { 6u8 }
        else { 7u8 }
    };
}

struct Encoder {
    buffer: ByteBuffer
}

impl Encoder {
    fn new_with_size(size: usize) -> Self {
        let mut buffer = ByteBuffer::new();
        buffer.resize(size);
        Encoder { buffer: buffer }
    }

    fn new() -> Self {
        let mut buffer = ByteBuffer::new();
        buffer.resize(ENCODER_BUFFER_SIZE);
        Encoder { buffer: buffer }
    }
}

impl Encoder {
    fn encode_byte_len(&self, input: u8) -> usize {
        return 1;
    }

    fn encode_byte(&mut self, input: u8) {
        if input > SINGLE_BYTE_MAX_VALUE {
            panic!("Byte value is greater than 0x7f.");
        } else {
            self.buffer.write_u8(input);
        }
    }

    fn encode_short_str_len(& self, input: &str) -> usize {
        return 1 + input.len();
    }

    fn encode_short_str(&mut self, input: &str) {
        if input.len() > SHORT_STRING_MAX_LEN {
            panic!("String length out of range 0-55.");
        } else {
            let prefix: u8 = SHORT_STRING_PREFIX_BASE + input.len() as u8;
            self.buffer.write_u8(prefix);
            self.buffer.write(input.as_bytes());
        }
    }

    fn encode_long_str_len(& self, input: &str) -> usize {
        let l = input.len() as u64;
        let l_total_byte = total_bytes!(l);
        return 1usize + l_total_byte as usize + input.len() as usize;
    }

    fn encode_long_str(&mut self, input: &str) {
        if input.len() <= SHORT_STRING_MAX_LEN {
            panic!("String length is no enough for encoding.");
        } else {
            let l = input.len() as u64;
            let l_total_byte = total_bytes!(l);

            let prefix: u8 = LONG_STRING_PREFIX_BASE + l_total_byte;

            self.buffer.write(&[prefix]);
            let len_bytes: [u8; 8] = unsafe { transmute(l.to_be()) };
            for i in 0..l_total_byte {
                self.buffer.write_u8(len_bytes[i as usize]);
            }
            self.buffer.write(input.as_bytes());
        }
    }

    fn encode_item_len(&self, input: &str) -> usize {
        if input.len() == 1 && input.as_bytes()[0usize] <= SINGLE_BYTE_MAX_VALUE {
            self.encode_byte_len( input.as_bytes()[0usize])
        } else if input.len() <= SHORT_STRING_MAX_LEN {
            self.encode_short_str_len(input)
        } else {
            self.encode_long_str_len(input)
        }
    }

    fn encode_item(&mut self, input: &str) {
        if input.len() == 1 && input.as_bytes()[0usize] <= SINGLE_BYTE_MAX_VALUE {
            self.encode_byte( input.as_bytes()[0usize]);
        } else if input.len() <= SHORT_STRING_MAX_LEN {
            self.encode_short_str(input);
        } else {
            self.encode_long_str(input);
        }
    }

    fn encode_list_len(&self, input: &RLP) -> usize {
        match input {
            &RLP::RLPItem { ref value } => {
                self.encode_item_len(value.as_str())
            },
            &RLP::RLPList { ref list } => {
                let mut total = 0usize;
                for elem in list {
                    total = total + self.encode_list_len(&elem);
                }
                if total <= SHORT_LIST_MAX_LEN {
                    1 + total
                } else {
                    let l_len = total_bytes!(total as u64);
                    1 + l_len as usize + total as usize
                }
            },
        }
    }

    fn encode_list(&mut self, input: &RLP) {
        match input {
            &RLP::RLPItem { ref value } => {
                self.encode_item(value.as_str());
            },
            &RLP::RLPList { ref list } => {
                let l = self.encode_list_len(input) as u64;
                if l <= SHORT_LIST_MAX_LEN as u64 {
                    let prefix: u8 = LONG_LIST_PREFIX_BASE + l as u8;
                    self.buffer.write_u8(prefix);
                    for elem in list {
                        self.encode_list(elem);
                    }
                } else {
                    let l_total_byte = total_bytes!(l);
                    let prefix: u8 = LONG_STRING_PREFIX_BASE + l_total_byte;
                    self.buffer.write_u8(prefix);

                    let len_bytes: [u8; 8] = unsafe { transmute(l.to_be()) };
                    for i in 0..l_total_byte {
                        self.buffer.write_u8(len_bytes[i as usize]);
                    }

                    for elem in list {
                        self.encode_list(elem);
                    }
                }
            },
        }
    }

    pub fn encode(&mut self, obj: &RLP) -> EncodedRLP {
        self.buffer.clear();
        let len = self.encode_list_len(obj);
        self.encode_list(obj);
        Vec::from_iter(self.buffer.to_bytes()[0..len].iter().cloned())
    }
}
