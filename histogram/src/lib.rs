use aya::{
    maps::{MapData, PerCpuHashMap},
    Pod,
};
use std::{hash::Hash, sync::Arc};

use prometheus::{
    core::{AtomicF64, Collector, Desc, GenericGaugeVec},
    proto, GaugeVec, Opts,
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
    buckets_metric: GenericGaugeVec<AtomicF64>,
}


impl<T: Key> Collector for Histogram<T> {
    fn desc(&self) -> Vec<&Desc> {
        self.buckets_metric.desc()
    }

    fn collect(&self) -> Vec<proto::MetricFamily> {
        self.map
            .iter()
            .filter_map(|row| match row {
                Ok(x) => Some(x),
                Err(_) => None,
            })
            .for_each(|(key, values)| {
                let label_keys = key.sub_key.get_label_values();
                let bucket = key.bucket.to_string();
                let mut str_label_keys: Vec<&str> = label_keys.iter().map(|x| x.as_str()).collect();
                str_label_keys.push(bucket.as_str());

                let total = values.iter().sum::<u64>();
                self.buckets_metric
                    .with_label_values(&str_label_keys)
                    .set(total as f64)
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

        let buckets_metric = GaugeVec::new(bucket_opts, &str_label_keys).unwrap();
        Histogram {
            map: Arc::from(map),
            buckets_metric,
            // phantom: PhantomData,
        }
    }
}
