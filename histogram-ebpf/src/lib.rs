#![no_std]
use aya_ebpf::maps::PerCpuHashMap;

#[inline(always)]
fn bpf_log2(mut v: u32) -> u32 {
    let mut r: u32;
    let mut shift: u32;
    r = ((v > 0xFFFF) as u32) << 4;
    v >>= r;
    shift = ((v > 0xFF) as u32) << 3;
    v >>= shift;
    r |= shift;
    shift = ((v > 0xF) as u32) << 2;
    v >>= shift;
    r |= shift;
    shift = ((v > 0x3) as u32) << 1;
    v >>= shift;
    r |= shift;
    r |= v >> 1;
    r
}

/// Return the log2(v) ceiled
/// It should match the equivalent function in BCC
fn bpf_log2l(v: u64) -> u32 {
    let lo: u32 = (v & 0xFFFFFFFF) as u32;
    let hi: u32 = (v >> 32) as u32;

    if hi != 0 {
        bpf_log2(hi) + 32 + 1
    } else {
        bpf_log2(lo) + 1
    }
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct Key<T: Sized> {
    pub bucket: u32,
    pub sub_key: T,
}

pub struct BpfHistogram<T> {
    map: PerCpuHashMap<Key<T>, u64>,
}

impl<T> BpfHistogram<T> {
    pub const fn with_max_entries(max_entries: u32, flags: u32) -> BpfHistogram<T> {
        BpfHistogram {
            map: PerCpuHashMap::with_max_entries(max_entries, flags),
        }
    }

    #[inline(always)]
    pub fn observe(&self, sub_key: T, value: u64) {
        let bucket = bpf_log2l(value);
        let key = Key { bucket, sub_key };

        unsafe {
            let counter = match self.map.get(&key) {
                Some(i) => i + 1,
                None => 1,
            };
            let _ = self.map.insert(&key, &counter, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bpf_log2l;

    #[test]
    fn bpf_log2_works() {
        assert_eq!(bpf_log2l(10), 4);
        assert_eq!(bpf_log2l(10000000), 24);
        assert_eq!(bpf_log2l(1000000000), 30);
        assert_eq!(bpf_log2l(100000000000), 37);
        assert_eq!(bpf_log2l(10000000000000), 44);
        assert_eq!(bpf_log2l(2043866223362000), 51);
    }
}
