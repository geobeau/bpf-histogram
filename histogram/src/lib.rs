use aya::{
    maps::{MapData, PerCpuHashMap},
    Pod,
};
use std::{collections::BTreeMap, collections::HashMap, hash::Hash, sync::Arc};

use prometheus::{
    core::{AtomicI64, Collector, Desc, GenericGaugeVec}, proto, IntGaugeVec, Opts
};

pub trait Key: Sized + Pod + Eq + PartialEq + Hash + Send + Sync {
    /// Return keys for the Prometheus label pairs:
    /// - Can be empty
    /// - Should be the same order and size as `get_label_values`
    fn get_label_keys() -> Vec<String>;
    /// Return values for the Prometheus label pairs:
    /// - Can be empty
    /// - Should be the same order as get_label_keys
    fn get_label_values(&self) -> Vec<String>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct KeyWrapper<T: Key> {
    pub bucket: u32,
    pub sub_key: T,
}

unsafe impl<T: Key> Pod for KeyWrapper<T> {}

#[derive(Clone)]
pub struct Histogram<T: Key> {
    map: Arc<PerCpuHashMap<MapData, KeyWrapper<T>, u64>>,
    // phantom: PhantomData<T>,
    buckets_metric: GenericGaugeVec<AtomicI64>,
}


impl<T: Key> Collector for Histogram<T> {
    fn desc(&self) -> Vec<&Desc> {
        self.buckets_metric.desc()
    }

    fn collect(&self) -> Vec<proto::MetricFamily> {
        // On the ebpf side, only the current bucket is incr, but prometheus expect
        // all bucket lower or equal to be incremented as well.
        // So `e` (equal) buckets needs to be transformed to `le` (lower equal)
        let mut sorted_bucket_index = HashMap::<T, BTreeMap<u32, u64>>::new();
        self.map
            .iter()
            .filter_map(|row| match row {
                Ok(x) => Some(x),
                Err(_) => None,
            })
            .for_each(|(key, values)| {
                let entry = sorted_bucket_index.entry(key.sub_key).or_insert_with(|| BTreeMap::new());
                let total = values.iter().sum::<u64>();
                entry.insert(key.bucket, total);
            });

        // Use the sorted_bucket_index to accumulate total to form `le` buckets
        sorted_bucket_index
            .iter()
            .for_each(|(key, bucket_map)| {
                let label_values = key.get_label_values();

                let mut total = 0;
                bucket_map.iter().for_each(|(bucket, value)| {
                    total += value;
                    // buckets are exposed as the exponant of a power of 2
                    let expanded_bucket = ((2 as u64).pow(*bucket)).to_string();
                    let mut str_label_values: Vec<&str> = label_values.iter().map(|x| x.as_str()).collect();
                    str_label_values.push(expanded_bucket.as_str());

                    self.buckets_metric
                        .with_label_values(&str_label_values)
                        .set(total as i64)
                });
            });
        self.buckets_metric.collect()
    }
}

impl<T: Key> Histogram<T> {
    pub fn new_from_map(map: PerCpuHashMap<MapData, KeyWrapper<T>, u64>) -> Histogram<T> {
        let bucket_opts = Opts::new("test_latency", "test counter help");
        let label_keys = T::get_label_keys();
        let mut str_label_keys: Vec<&str> = label_keys.iter().map(|x| x.as_str()).collect();
        str_label_keys.push("le");  // le contains the lower/equal buckets for histogram

        let buckets_metric = IntGaugeVec::new(bucket_opts, &str_label_keys).unwrap();
        Histogram {
            map: Arc::from(map),
            buckets_metric,
            // phantom: PhantomData,
        }
    }
}
