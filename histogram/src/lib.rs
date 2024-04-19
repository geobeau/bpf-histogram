use aya::{
    maps::{MapData, PerCpuHashMap},
    Pod,
};
use std::{hash::Hash, sync::Arc};
use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};

use prometheus::{core::{Atomic, AtomicF64, Collector, Desc, GenericGauge, GenericGaugeVec}, proto, Encoder, Gauge, GaugeVec, Opts, Registry, TextEncoder};


pub trait Key: Sized + Pod + Eq + PartialEq + Hash + Send + Sync {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct KeyWrapper<T: Sized + Pod + Eq + PartialEq + Hash + Send + Sync> {
    pub bucket: u32,
    pub sub_key: T,
}

unsafe impl<T: Sized + Pod + Eq + PartialEq + Hash + Send + Sync> Pod for KeyWrapper<T> {}

#[derive(Clone)]
pub struct Histogram<T: Sized + Pod + Eq + PartialEq + Hash + Send + Sync> {
    map: Arc<PerCpuHashMap<MapData, KeyWrapper<T>, u64>>,
    // phantom: PhantomData<T>,
    buckets_metric: GenericGaugeVec<AtomicF64>,
}

pub struct PerKeyHistogram {
    buckets: BTreeMap<u64, u64>,
}

impl PerKeyHistogram {
    pub fn new_from_map() {}
}

impl<T: Sized + Pod + Eq + PartialEq + Hash + Send + Sync> Collector for Histogram<T> {
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
                println!("{:?}", values);
                let total = values.iter().sum::<u64>();
                self.buckets_metric.with_label_values(&[key.bucket.to_string().as_str()]).set(total as f64)
            });
        self.buckets_metric.collect()
    }
}

impl<T: Sized + Pod + Eq + PartialEq + Hash + Send + Sync> Histogram<T> {
    pub fn new_from_map(map: PerCpuHashMap<MapData, KeyWrapper<T>, u64>) -> Histogram<T> {
        let bucket_opts = Opts::new("test_latency", "test counter help");
        let buckets_metric = GaugeVec::new(bucket_opts, &["le"]).unwrap();
        Histogram {
            map: Arc::from(map),
            buckets_metric,
            // phantom: PhantomData,
        }
    }
}
