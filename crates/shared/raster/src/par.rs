//! Work split over threads natively, one piece after another in WASM (the
//! browser's analysis worker is one thread; docs/adr/0231 §11). Results come
//! back in order, so every target writes the same bytes.

/// `f` of each item, in order; at most `threads` at once.
pub fn map<T: Sync, R: Send>(threads: usize, items: &[T], f: &(dyn Fn(&T) -> R + Sync)) -> Vec<R> {
    #[cfg(not(target_arch = "wasm32"))]
    if threads > 1 && items.len() > 1 {
        let per = items.len().div_ceil(threads.min(items.len()));
        return std::thread::scope(|s| {
            let handles: Vec<_> = items
                .chunks(per)
                .map(|chunk| s.spawn(move || chunk.iter().map(f).collect::<Vec<R>>()))
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
                .collect()
        });
    }
    let _ = threads;
    items.iter().map(f).collect()
}

/// `f(first row, rows)` over the rows of `out` (`row` values each), cut into
/// at most `threads` runs of whole rows.
pub fn rows<T: Send>(
    threads: usize,
    out: &mut [T],
    row: usize,
    f: &(dyn Fn(usize, &mut [T]) + Sync),
) {
    let n = out.len().checked_div(row).unwrap_or(0);
    #[cfg(not(target_arch = "wasm32"))]
    if threads > 1 && n > 1 {
        let per = n.div_ceil(threads.min(n));
        std::thread::scope(|s| {
            for (k, chunk) in out.chunks_mut(per * row).enumerate() {
                s.spawn(move || f(k * per, chunk));
            }
        });
        return;
    }
    let _ = (threads, n);
    f(0, out);
}

/// Threads for an analysis on this machine: the cores but one, at least 1, at most 8.
pub fn threads() -> usize {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1))
            .clamp(1, 8)
    }
    #[cfg(target_arch = "wasm32")]
    {
        1
    }
}
