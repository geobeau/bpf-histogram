use aya::{
    maps::{MapData, PerCpuHashMap},
    Pod,
};
use std::hash::Hash;
use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Key<T: Sized + Pod + Eq + PartialEq + Hash> {
    pub bucket: u32,
    pub sub_key: T,
}

unsafe impl<T: Sized + Pod + Eq + PartialEq + Hash> Pod for Key<T> {}

pub struct Histogram<T: Pod + Eq + PartialEq + Hash> {
    map: PerCpuHashMap<MapData, Key<T>, u64>,
    phantom: PhantomData<T>,
}

pub struct PerKeyHistogram {
    buckets: BTreeMap<u64, u64>,
}

impl PerKeyHistogram {
    pub fn new_from_map() {}
}

impl<T: Pod + Eq + PartialEq + Hash> Histogram<T> {
    pub fn new_from_map(map: PerCpuHashMap<MapData, Key<T>, u64>) -> Histogram<T> {
        Histogram {
            map,
            phantom: PhantomData,
        }
    }

    pub fn export_to_le_histogram(&self) -> HashMap<T, Vec<(u64, u64)>> {
        let mut per_key_histogram: HashMap<T, Vec<(u64, u64)>> = HashMap::new();
        self.map.iter().for_each(|x| println!("{:?}", x.unwrap().1));
        self.map
            .iter()
            .filter_map(|row| match row {
                Ok(x) => Some(x),
                Err(_) => None,
            })
            .for_each(|(key, values)| {
                println!("{:?}", values);
                let total = values.iter().sum::<u64>();
                per_key_histogram
                    .entry(key.sub_key)
                    .and_modify(|val| val.push((key.bucket.into(), total)))
                    .or_insert(vec![(key.bucket.into(), total)]);
            });
        per_key_histogram
    }
}
