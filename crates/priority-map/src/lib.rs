/// Adds new functions to StorageLinkedMap
/// https://substrate.dev/rustdocs/master/src/frame_support/storage/generator/linked_map.rs.html
///

use codec::{FullCodec, Encode, Decode, EncodeLike, Ref};
use frame_support::{storage::{self, unhashed, StorageLinkedMap}, hash::{StorageHasher, Twox128}, traits::Len};
use sp_std::{prelude::*, marker::PhantomData};


impl<K, V, G> storage::StorageLinkedMap<K, V> for G
where
    K: FullCodec,
    V: FullCodec,
    G: StorageLinkedMap<K, V>
{
    fn insert_sorted() -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
