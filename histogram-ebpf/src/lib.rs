#![no_std]

use aya_bpf::maps::PerCpuHashMap;



#[inline(always)]
fn bpf_log2(mut v: u64) -> u64 {
    let mut r: u64;
    let mut shift: u64;
    r = ((v > 0xFFFF) as u64) << 4; v >>= r;
    shift = ((v > 0xFF) as u64) << 3; v >>= shift; r |= shift;
    shift = ((v > 0xF) as u64) << 2; v >>= shift; r |= shift;
    shift = ((v > 0x3) as u64) << 1; v >>= shift; r |= shift;
    r |= (v >> 1) as u64;
    return r;
}


#[repr(C)]
pub struct Key<T: Sized> {
    pub bucket: u32,
    pub sub_key: T
}


pub struct BpfHistogram<T> {
    map: PerCpuHashMap<Key<T>, u64>
}

impl<T> BpfHistogram<T> {
    pub const fn with_max_entries(max_entries: u32, flags: u32) -> BpfHistogram<T> {
        return BpfHistogram {
            map: PerCpuHashMap::with_max_entries(max_entries, flags)
        }
    }

    #[inline(always)]
    pub fn observe(&self, sub_key: T, value: u64) {
        let bucket = bpf_log2(value);
        let key = Key { bucket: bucket as u32, sub_key };

        unsafe {
            let counter = match self.map.get(&key) {
                Some(i) => i + 1,
                None => 1,
            };
            let _ = self.map.insert(&key, &counter, 0);
        }
    }
}
