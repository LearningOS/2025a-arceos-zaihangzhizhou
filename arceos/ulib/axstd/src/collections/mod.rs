mod hash_map;
pub use hash_map::HashMap;

pub fn new_hashmap<K, V>() -> HashMap<K, V> 
where
    K: core::hash::Hash + Eq,
{
    HashMap::new()
}