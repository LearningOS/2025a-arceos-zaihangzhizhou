use core::hash::{BuildHasher, Hash, Hasher};
use alloc::vec::Vec;
use alloc::boxed::Box;
use arceos_api::random as api;

#[derive(Clone)]
pub struct RandomState(u64);

impl RandomState {
    pub fn new() -> Self {
        Self(api::ax_random() as u64)
    }
}

impl Default for RandomState {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildHasher for RandomState {
    type Hasher = SimpleHasher;
    
    fn build_hasher(&self) -> SimpleHasher {
        SimpleHasher::new(self.0)
    }
}

pub struct SimpleHasher(u64);

impl SimpleHasher {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
}

impl Hasher for SimpleHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.0;
        for &byte in bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }
    
    fn finish(&self) -> u64 {
        self.0
    }
}

struct Entry<K, V> {
    key: K,
    value: V,
    next: Option<Box<Entry<K, V>>>,
}

pub struct HashMap<K, V, S = RandomState> 
where
    S: BuildHasher,
{
    buckets: Vec<Option<Box<Entry<K, V>>>>,
    len: usize,
    hasher: S,
}

impl<K, V> HashMap<K, V> 
where
    K: Hash + Eq,
{
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, V, S> HashMap<K, V, S> 
where
    K: Hash + Eq,
    S: BuildHasher,
{
    pub fn with_hasher(hasher: S) -> Self {
        Self::with_capacity_and_hasher(16, hasher)
    }
    
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        let capacity = capacity.max(1);
        let mut buckets = Vec::with_capacity(capacity);
        buckets.resize_with(capacity, || None);
        
        Self { buckets, len: 0, hasher }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.len >= self.buckets.len() * 3 / 4 {
            self.resize();
        }
        
        let index = self.hash_index(&key);
        let bucket = &mut self.buckets[index];
        
        let mut current: Option<&mut Box<Entry<K, V>>> = bucket.as_mut();
        while let Some(entry) = current {
            if entry.key == key {
                return Some(core::mem::replace(&mut entry.value, value));
            }
            current = entry.next.as_mut();
        }
        
        let new_entry = Box::new(Entry {
            key,
            value,
            next: bucket.take(),
        });
        *bucket = Some(new_entry);
        self.len += 1;
        None
    }
    
    pub fn get(&self, key: &K) -> Option<&V> {
        let index = self.hash_index(key);
        let mut current: Option<&Box<Entry<K, V>>> = self.buckets[index].as_ref();
        
        while let Some(entry) = current {
            if &entry.key == key {
                return Some(&entry.value);
            }
            current = entry.next.as_ref();
        }
        None
    }
    
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = self.hash_index(key);
        let bucket = &mut self.buckets[index];
        
        if let Some(mut entry) = bucket.take() {
            if entry.key == *key {
                *bucket = entry.next.take();
                self.len -= 1;
                return Some(entry.value);
            }
            
            let mut prev = &mut *entry;
            while let Some(mut current) = prev.next.take() {
                if current.key == *key {
                    prev.next = current.next.take();
                    self.len -= 1;
                    return Some(current.value);
                } else {
                    prev.next = Some(current);
                    prev = prev.next.as_mut().unwrap();
                }
            }
            *bucket = Some(entry);
        }
        None
    }
    
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            buckets: &self.buckets,
            index: 0,
            current: None,
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }
    
    pub fn len(&self) -> usize {
        self.len
    }
    
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            *bucket = None;
        }
        self.len = 0;
    }
    
    fn hash_index(&self, key: &K) -> usize {
        let mut hasher = self.hasher.build_hasher();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.buckets.len()
    }
    
    fn resize(&mut self) {
        let new_capacity = self.buckets.len() * 2;
        let mut new_buckets = Vec::with_capacity(new_capacity);
        new_buckets.resize_with(new_capacity, || None);
        
        let old_buckets = core::mem::replace(&mut self.buckets, new_buckets);
        self.len = 0;
        
        for bucket in old_buckets.into_iter().flatten() {
            let mut current = Some(bucket);
            while let Some(mut entry) = current.take() {
                let key = entry.key;
                let value = entry.value;
                
                current = entry.next.take();
                
                self.insert(key, value);
            }
        }
        
    }
}

pub struct Iter<'a, K, V> {
    buckets: &'a Vec<Option<Box<Entry<K, V>>>>,
    index: usize,
    current: Option<&'a Entry<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(entry) = self.current {
                self.current = entry.next.as_ref().map(|b| &**b);
                return Some((&entry.key, &entry.value));
            }
            
            if self.index >= self.buckets.len() {
                return None;
            }
            
            self.current = self.buckets[self.index].as_ref().map(|b| &**b);
            self.index += 1;
        }
    }
}