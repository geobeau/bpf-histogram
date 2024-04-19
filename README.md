[ebpf-crates-badge]: https://img.shields.io/crates/v/ebpf-histogram-ebpf.svg?style=for-the-badge&logo=rust
[ebpf-crates-url]: https://crates.io/crates/ebpf-histogram-ebpf
[crates-badge]: https://img.shields.io/crates/v/ebpf-histogram.svg?style=for-the-badge&logo=rust
[crates-url]: https://crates.io/crates/ebpf-histogram
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge

# ebpf-histogram

Provides a library to create Prometheus histogram directly from ebpf data structures.
It depends on aya-rs and Prometheus, and requires familiarity with both APIs.

# Usage

## Usage in ebpf: `ebpf-histogram-ebpf`

[![Crates.io][ebpf-crates-badge]][ebpf-crates-url] ![License][license-badge]

In `cargo.toml`
```rust
ebpf-histogram-ebpf = "0.1.0"
```

```rust
use ebpf_histogram_ebpf::BpfHistogram;

#[derive(Copy, Clone)]
#[repr(C)] // You need to make sure the format will be similar betweem user space and ebpf code
pub struct DiskLatencyHistogramKey {
    pub major: i32,
    pub minor: i32
}

#[map]
static BLOCK_HISTOGRAM: BpfHistogram<DiskLatencyHistogramKey> = BpfHistogram::with_max_entries(1000, 0);


#[btf_tracepoint(function="block_rq_complete")]
pub fn block_rq_complete(ctx: BtfTracePointContext) -> u32 {
    let req: *const vmlinux::request = unsafe { ctx.arg(0) };

    unsafe {
        let timestamp = bpf_ktime_get_ns();
        let disk: vmlinux::gendisk = *((*(*req).q).disk);
        let latency = timestamp - (*req).io_start_time_ns;
        let flags = (*req).cmd_flags & REQ_OP_MASK;
        // This is the only code that matters. It tries to copy the API of Prometheus histogram
        BLOCK_HISTOGRAM.observe(DiskLatencyHistogramKey{ major: disk.major, minor: disk.minors }, latency);
        info!(&ctx, "complete disk {}.{} -> Latency: {}us, (flags: {})", disk.major,disk.minors, latency / 1000, flags);
    }
    return 0
}
```

## Usage in userspace: `ebpf-histogram`


[![Crates.io][crates-badge]][crates-url] ![License][license-badge]


In `cargo.toml`
```rust
ebpf-histogram = "0.1.0"
```

```rust
use prometheus::{Opts, Registry, TextEncoder};
use ebpf_histogram::{Histogram, Key, KeyWrapper};


#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
// #[derive(Key)]
#[repr(C)]
pub struct DiskLatencyHistogramKey {
    pub major: i32,
    pub minor: i32
}

unsafe impl Send for DiskLatencyHistogramKey {}
unsafe impl Sync for DiskLatencyHistogramKey {}
unsafe impl Pod for DiskLatencyHistogramKey {}
impl Key for DiskLatencyHistogramKey {
    // Inform which labels pairs will be exposed by Prometheus
    fn get_label_keys() -> Vec<String> {
        return vec!["major".to_string(), "minor".to_string()]
    }

    // Transform the key into labels values every time the collection of metrics is done
    fn get_label_values(&self) -> Vec<String> {
        return vec![self.major.to_string(), self.minor.to_string()]
    }
}


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    /*
    Main code
    */

    // Get the map in common with ebpf code
    let map: PerCpuHashMap<_, KeyWrapper<DiskLatencyHistogramKey>, u64> = PerCpuHashMap::try_from(
        bpf.take_map("BLOCK_HISTOGRAM").expect("failed to map BLOCK_HISTOGRAM"),
    )?;

    // Define Prometheus configuration for the metric
    let bucket_opts = Opts::new("test_latency", "test counter help");
    let histogram: Histogram<DiskLatencyHistogramKey> = Histogram::new_from_map(map, bucket_opts);

    // The histogran implement Collect so it can be used like a regular Prometheus metric
    let r = Registry::new();
    r.register(Box::new(histogram)).unwrap();

    // See the documentation of Rust Prometheus to see how to use the metric then
    let mut buffer = String::new();
    let encoder = TextEncoder::new();
    let metric_families = r.gather();
    encoder.encode_utf8(&metric_families, &mut buffer).unwrap();
    println!("{}", buffer);

}
```


