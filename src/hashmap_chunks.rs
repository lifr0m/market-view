use std::collections::{hash_map::IntoIter, HashMap};
use std::hash::Hash;

pub(crate) struct HashMapChunks<K, V> {
    iter: IntoIter<K, V>,
    chunk_size: usize,
}

impl<K, V> HashMapChunks<K, V> {
    pub(crate) fn new(map: HashMap<K, V>, size: usize) -> Self {
        Self { iter: map.into_iter(), chunk_size: size }
    }
}

impl<K, V> Iterator for HashMapChunks<K, V>
where
    K: Eq + Hash
{
    type Item = HashMap<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter.len() == 0 {
            None
        } else {
            let chunk_size = std::cmp::min(self.iter.len(), self.chunk_size);
            Some(HashMap::from_iter(
                (0..chunk_size)
                    .map(|_| self.iter.next().unwrap())
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashmap_chunks() {
        let map = HashMap::from([
            (1, 2),
            (3, 4),
            (5, 6),
        ]);
        let mut test_map = map.clone();
        let mut chunks = HashMapChunks::new(map, 2);
        
        for len in [2, 1] {
            let chunk = chunks.next().unwrap();
            assert_eq!(chunk.len(), len);
            for key in chunk.into_keys() {
                test_map.remove(&key).unwrap();
            }
        }
        assert_eq!(chunks.next(), None);
    }
}
