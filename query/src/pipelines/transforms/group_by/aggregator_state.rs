use common_datablocks::{HashMethod, HashMethodFixedKeys, HashMethodSerializer};
use common_datavalues::DFNumericType;
use std::fmt::Debug;
use std::hash::Hash;

use crate::common::{HashMap, HashTableEntity, KeyValueEntity, HashTableKeyable, HashTableIter, HashMapIterator};
use std::ptr::NonNull;
use std::alloc::Layout;
use bumpalo::Bump;
use crate::pipelines::transforms::group_by::keys_ref::KeysRef;

pub trait AggregatorState<Method: HashMethod> {
    type HashKeyState: HashTableKeyable;

    fn len(&self) -> usize;

    fn alloc_layout(&self, layout: Layout) -> NonNull<u8>;

    fn iter(&self) -> HashMapIterator<Self::HashKeyState, usize>;

    fn insert_key(&mut self, key: &Method::HashKey, inserted: &mut bool) -> *mut KeyValueEntity<Self::HashKeyState, usize>;
}

// TODO: Optimize the type with length below 2
pub struct FixedKeysAggregatorState<T> where
    T: DFNumericType,
    T::Native: std::cmp::Eq + Clone + Debug,
    HashMethodFixedKeys<T>: HashMethod<HashKey=T::Native>,
    <HashMethodFixedKeys<T> as HashMethod>::HashKey: HashTableKeyable
{
    pub area: Bump,
    pub data: HashMap<T::Native, usize>,
}

impl<T> AggregatorState<HashMethodFixedKeys<T>> for FixedKeysAggregatorState<T> where
    T: DFNumericType,
    T::Native: std::cmp::Eq + Hash + Clone + Debug,
    HashMethodFixedKeys<T>: HashMethod<HashKey=T::Native>,
    <HashMethodFixedKeys<T> as HashMethod>::HashKey: HashTableKeyable
{
    type HashKeyState = <HashMethodFixedKeys<T> as HashMethod>::HashKey;

    #[inline(always)]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    fn alloc_layout(&self, layout: Layout) -> NonNull<u8> {
        self.area.alloc_layout(layout)
    }

    #[inline(always)]
    fn iter(&self) -> HashMapIterator<<HashMethodFixedKeys<T> as HashMethod>::HashKey, usize> {
        self.data.iter()
    }

    #[inline(always)]
    fn insert_key(&mut self, key: &<HashMethodFixedKeys<T> as HashMethod>::HashKey, inserted: &mut bool) -> *mut KeyValueEntity<<HashMethodFixedKeys<T> as HashMethod>::HashKey, usize> {
        self.data.insert_key(key, inserted)
    }
}

pub struct SerializedKeysAggregatorState {
    pub keys_area: Bump,
    pub state_area: Bump,
    pub data_state_map: HashMap<KeysRef, usize>,
}

impl AggregatorState<HashMethodSerializer> for SerializedKeysAggregatorState {
    type HashKeyState = KeysRef;

    fn len(&self) -> usize {
        self.data_state_map.len()
    }

    fn alloc_layout(&self, layout: Layout) -> NonNull<u8> {
        self.state_area.alloc_layout(layout)
    }

    fn iter(&self) -> HashMapIterator<KeysRef, usize> {
        self.data_state_map.iter()
    }

    fn insert_key(&mut self, keys: &Vec<u8>, inserted: &mut bool) -> *mut KeyValueEntity<KeysRef, usize> {
        let mut keys_ref = KeysRef::create(keys.as_ptr() as usize, keys.len());
        let state_entity = self.data_state_map.insert_key(&keys_ref, inserted);

        if *inserted {
            unsafe {
                // Keys will be destroyed after call we need copy the keys to the memory pool.
                let global_keys = self.keys_area.alloc_slice_copy(keys);
                let inserted_hash = state_entity.get_hash();
                keys_ref.address = global_keys.as_ptr() as usize;
                // TODO: maybe need set key method.
                state_entity.set_key_and_hash(&keys_ref, inserted_hash)
            }
        }

        state_entity
    }
}

