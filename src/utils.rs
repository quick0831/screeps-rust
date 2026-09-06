use std::cmp::Ordering;
use std::f64::consts::FRAC_1_PI;
use std::f64::consts::PI;

/// Associates an un-compared value with a compared key, intended for use in `BinaryHeap`
pub struct KeyCmp<K: Ord, V> {
    pub key: K,
    pub value: V,
}

impl<K: Ord, V> PartialEq for KeyCmp<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}

impl<K: Ord, V> Eq for KeyCmp<K, V> {}

impl<K: Ord, V> PartialOrd for KeyCmp<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Ord, V> Ord for KeyCmp<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

// START OF const-evaluable sin function
// Source: https://gist.github.com/sug0/b5eb2c58be74f7cda230b8c1e1994670

#[allow(unused)]
pub const fn to_radians(x: f64) -> f64 {
    x * (PI / 180.0)
}

// https://www.gamedev.net/forums/topic/621589-extremely-fast-sin-approximation/
pub const fn fast_sin_partial(mut x: f64) -> f64 {
    let (mut y, mut z) = (x, x);
    z *= FRAC_1_PI;
    z += 6755399441055744.0;
    let mut k = unsafe {
        let p: *const i32 = &z as *const _ as *const _;
        *p
    };
    z = k as f64;
    z *= PI;
    x -= z;
    y *= x;
    z = 0.0073524681968701;
    z *= y;
    z -= 0.1652891139701474;
    z *= y;
    z += 0.9996919862959676;
    x *= z;
    k &= 1;
    k += k;
    z = k as f64;
    z *= x;
    x -= z;
    x
}

pub const fn fast_sin(x: f64) -> f64 {
    let p = (x * FRAC_1_PI).round();
    let n = if p as i32 % 2 == 0 { 1. } else { -1. };
    fast_sin_partial((x - PI * p) * n)
}

// END OF const-evaluable sin function

pub const fn fir_low_pass<const N: usize>(rad: f64) -> [f64; N] {
    let mut buf = [0.; N];
    let mut sum = 0.;
    let half = (N - 1) / 2;

    // calculate sinc
    let mut i = 0;
    while i < N {
        let n = (i as isize - half as isize) as f64;
        buf[i] = if i == half {
            rad * FRAC_1_PI
        } else {
            fast_sin(rad * n) * FRAC_1_PI / n
        };
        sum += buf[i];
        i += 1;
    }

    // adjust DC gain to 1
    i = 0;
    while i < N {
        buf[i] /= sum;
        i += 1;
    }

    buf
}
