//! Stable sorts that share one sorting routine.
//!
//! `slice::sort_by` / `sort_by_key` are generic over the element type and the comparator,
//! so every call site carries its own copy of the std stable sort (a few KB each on wasm).
//! These sort a `u32` index permutation through one non-generic routine and then move the
//! elements into place, so the result is exactly what the std stable sort gives (equal
//! elements keep their order) while the sort itself exists once in the binary.

use core::cmp::Ordering;

/// Same result as `v.sort_by(cmp)` (stable).
pub fn sort_by<T>(v: &mut [T], mut cmp: impl FnMut(&T, &T) -> Ordering) {
    if v.len() < 2 {
        return;
    }
    let mut idx: Vec<u32> = (0..v.len() as u32).collect();
    {
        let v: &[T] = v;
        sort_idx(&mut idx, &mut |a, b| cmp(&v[a as usize], &v[b as usize]));
    }
    permute(v, &idx);
}

/// Same result as `v.sort_by_key(key)` (stable).
pub fn sort_by_key<T, K: Ord>(v: &mut [T], mut key: impl FnMut(&T) -> K) {
    sort_by(v, |a, b| key(a).cmp(&key(b)));
}

/// Same result as `v.sort()` (stable).
pub fn sort<T: Ord>(v: &mut [T]) {
    sort_by(v, T::cmp);
}

#[inline(never)]
fn sort_idx(idx: &mut [u32], cmp: &mut dyn FnMut(u32, u32) -> Ordering) {
    idx.sort_by(|a, b| cmp(*a, *b));
}

/// Puts the element that was at `idx[i]` at position `i`, for every `i`.
fn permute<T>(v: &mut [T], idx: &[u32]) {
    for i in 0..idx.len() {
        // Earlier swaps moved the element from `idx[i]` along its cycle; follow it.
        let mut j = idx[i] as usize;
        while j < i {
            j = idx[j] as usize;
        }
        v.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(s: &mut u64) -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *s >> 33
    }

    #[test]
    fn matches_std_stable_sort() {
        let mut s = 7u64;
        for n in 0..200usize {
            for m in [1u64, 3, 1000] {
                let v: Vec<(u64, usize)> = (0..n).map(|i| (lcg(&mut s) % m, i)).collect();
                let mut a = v.clone();
                a.sort_by_key(|x| x.0);
                let mut b = v.clone();
                sort_by_key(&mut b, |x| x.0);
                assert_eq!(a, b, "n {n} m {m}");
                let mut c = v.clone();
                sort_by(&mut c, |x, y| y.0.cmp(&x.0));
                let mut d = v;
                d.sort_by(|x, y| y.0.cmp(&x.0));
                assert_eq!(c, d);
            }
        }
        let mut w = vec!["b".to_string(), "a".into(), "c".into(), "a".into()];
        sort(&mut w);
        assert_eq!(w, ["a", "a", "b", "c"]);
    }
}
