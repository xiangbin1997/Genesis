extern crate common;
extern crate rlp;

use std::fmt;
use std::iter::Iterator;
use self::common::hash::*;
use self::common::rust_base58::{ToBase58, FromBase58};

use self::rlp::RLPSerialize;
use self::rlp::types::*;
use self::rlp::encoder::*;
use self::rlp::decoder::*;

pub type TrieKey = Hash;
pub type EncodedPath = Vec<u8>;

#[inline]
fn from_slice_to_key(bytes: &Vec<u8>) -> TrieKey {
    let mut a = [0u8; 32];
    for i in 0..a.len() {
        // Panics if not enough input
        a[i] = bytes[i];
    }
    a
}

#[inline]
fn from_slice_to_branch(keys: &Vec<TrieKey>) -> [TrieKey; 16] {
    let mut a = [[0u8; 32]; 16];
    for i in 0..a.len() {
        // Panics if not enough input
        a[i] = keys[i];
    }
    a
}

#[inline]
pub fn nibble2vec(nibble: &Vec<u8>) -> Vec<u8> {
    if nibble.len() % 2 != 0 {
        panic!("Invalid nibble length");
    }
    let mut output: Vec<u8> = vec![];
    let mut i = 0usize;
    loop {
        if i + 2usize > nibble.len() { break; }
        if nibble[i] >= 16u8 {
            panic!("Invalid nibble entry");
        }
        output.append(&mut vec![nibble[i] * 16u8 + nibble[i + 1usize]]);
        i = i + 2usize;
    }
    output
}

#[inline]
pub fn vec2nibble(vec: &Vec<u8>) -> Vec<u8> {
    let mut output: Vec<u8> = vec![];
    for i in (0usize .. vec.len()) {
        let byte: u8 = vec[i];
        output.append(&mut vec![ byte / 16u8, byte % 16u8]);
    }
    output
}

#[inline]
pub fn encode_path(nibble: &Vec<u8>, terminated: bool) -> EncodedPath {
    let is_odd = (nibble.len() % 2 != 0);
    if !is_odd && !terminated {
        let mut tmp = vec![0u8, 0u8];
        tmp.append(&mut nibble.clone());
        nibble2vec(&tmp)
    } else if is_odd && !terminated {
        let mut tmp = vec![1u8];
        tmp.append(&mut nibble.clone());
        nibble2vec(&tmp)
    } else if !is_odd && terminated {
        let mut tmp = vec![2u8, 0u8];
        tmp.append(&mut nibble.clone());
        nibble2vec(&tmp)
    } else if is_odd && terminated {
        let mut tmp = vec![3u8];
        tmp.append(&mut nibble.clone());
        nibble2vec(&tmp)
    } else {
        nibble2vec(&vec![])
    }
}

#[inline]
pub fn decode_path(encoded_path: &Vec<u8>) -> (Vec<u8>, bool) {
    let prefix = encoded_path[0] / 16u8;
    match prefix {
        0u8 => {
            let nibble = vec2nibble(encoded_path);
            (nibble[2 .. nibble.len()].to_vec(), false)
        },
        1u8 => {
            let nibble = vec2nibble(encoded_path);
            (nibble[1 .. nibble.len()].to_vec(), false)
        },
        2u8 => {
            let nibble = vec2nibble(encoded_path);
            (nibble[2 .. nibble.len()].to_vec(), true)
        },
        3u8 => {
            let nibble = vec2nibble(encoded_path);
            (nibble[1 .. nibble.len()].to_vec(), true)
        }
        _ => {
            panic!("Invalid prefix");
        }
    }
}

#[derive(Debug, Clone)]
pub enum TrieNode<T: RLPSerialize> {
    EMPTY,
    BranchNode { branches: [TrieKey; 16], value: T },
    ExtensionNode { encoded_path: EncodedPath, key: TrieKey },
    LeafNode { encoded_path: EncodedPath, value: T }
}

impl<T: RLPSerialize> RLPSerialize for TrieNode<T> {
    fn serialize(&self) -> Result<RLP, RLPError> {
        match self {
            &TrieNode::EMPTY => {
                Err(RLPError::RLPEncodingErrorUnencodable)
            },
            &TrieNode::BranchNode{ ref branches, ref value } => {
                let mut value_item = value.serialize()?;
                let mut rlp_list: Vec<RLP> = vec![];
                for elem in branches {
                    let elem_str_r = String::from_utf8(vec2nibble(&elem.to_vec()).to_vec());
                    match elem_str_r {
                        Ok(r) => {
                            let elem_item = RLP::RLPItem { value: r };
                            rlp_list.append(&mut vec![elem_item]);
                        },
                        Err(e) => {
                            return Err(RLPError::RLPErrorUTF8);
                        }
                    }
                }
                rlp_list.append(&mut vec![value_item]);
                Ok(RLP::RLPList { list: rlp_list })
            },
            &TrieNode::ExtensionNode{ ref encoded_path, ref key } => {
                let path_str_r = String::from_utf8(vec2nibble(encoded_path));
                let key_str_r = String::from_utf8((vec2nibble(&key.to_vec()).to_vec()).to_vec());
                match (path_str_r, key_str_r) {
                    (Ok(l), Ok(r)) => {
                        let list = vec![RLP::RLPItem { value: "e".to_string() }, RLP::RLPItem { value: l }, RLP::RLPItem { value: r }];
                        Ok(RLP::RLPList { list: list })
                    }
                    _ => {
                        Err(RLPError::RLPErrorUnknown)
                    }
                }
            },
            &TrieNode::LeafNode{ ref encoded_path, ref value } => {
                let path_str_r = String::from_utf8(vec2nibble(encoded_path).to_vec());
                let value_rlp_item = value.serialize();
                match (path_str_r, value_rlp_item) {
                    (Ok(l), Ok(r)) => {
                        let list = vec![RLP::RLPItem { value: l }, r];
                        Ok(RLP::RLPList { list: list })
                    }
                    _ => Err(RLPError::RLPErrorUnknown)
                }
            },
        }
    }

    fn deserialize(rlp: &RLP) -> Result<Self, RLPError> {
       match rlp {
           &RLP::RLPList { ref list } => {
               match list.len() {
                   //LeafNode
                   2usize => {
                       let path_item = &list[0];
                       let value_item = &list[1];
                       match (path_item, value_item)  {
                           (&RLP::RLPItem { value: ref path }, rlp) => {
                               Ok(TrieNode::LeafNode {
                                   encoded_path: nibble2vec(&path.as_bytes().to_vec()),
                                   value: match T::deserialize(rlp) {
                                       Ok(r) => r,
                                       _ => { return Err(RLPError::RLPErrorUnknown); }
                                   }
                               })
                           },
                           _ => Err(RLPError::RLPErrorUnknown)
                       }
                   },
                   //ExtensionNode
                   3usize => {
                       let path_item = &list[1];
                       let value_item = &list[2];
                       match (path_item, value_item)  {
                           (&RLP::RLPItem { value: ref path }, &RLP::RLPItem { value: ref key }) => {
                               Ok(TrieNode::ExtensionNode {
                                   encoded_path: nibble2vec(&path.as_bytes().to_vec()),
                                   key: from_slice_to_key(&nibble2vec(&key.as_bytes().to_vec()))
                               })
                           },
                           _ => Err(RLPError::RLPErrorUnknown)
                       }
                   },
                   //BranchNode
                   17usize => {
                       let mut buffer: Vec<TrieKey> = vec![];
                       let mut index= 0usize;
                       for iter in list {
                           if index == 16 { break; }
                           match iter {
                               &RLP::RLPItem { ref value } => {
                                   let key = from_slice_to_key(&nibble2vec(&value.as_bytes().to_vec()));
                                   buffer.append(&mut vec![key]);
                               },
                               _ => { return Err(RLPError::RLPErrorUnknown); }
                           }
                           index = index + 1;
                       }
                       let value_ref = &list[index];
                       let value = T::deserialize(value_ref)?;
                       Ok(TrieNode::BranchNode { branches: from_slice_to_branch(&buffer), value: value })
                   },
                   _ => Err(RLPError::RLPErrorUnknown)
               }
           },
           &RLP::RLPItem { ref value } => Err(RLPError::RLPErrorUnknown),
       }
    }
}