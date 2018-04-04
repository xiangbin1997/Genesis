extern crate gen_core;
extern crate rlp;

use gen_core::mpt::trie::*;
use rlp::RLPSerialize;

fn main() {
    let mut test = Trie::new([0u8; 32]);
    test.update(&"123".as_bytes().to_vec(), &"test".to_string());
    test.update(&"124".as_bytes().to_vec(), &"test".to_string());
    test.update(&"125".as_bytes().to_vec(), &"test".to_string());
    let test_v = test.get(&"123".as_bytes().to_vec());
}