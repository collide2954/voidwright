use std::num::NonZeroUsize;
use std::thread;

#[must_use]
pub fn map<T: Sync, U: Send>(items: &[T], work: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let size = share(items.len());
    thread::scope(|scope| {
        let workers: Vec<_> = items
            .chunks(size)
            .map(|chunk| scope.spawn(|| chunk.iter().map(&work).collect::<Vec<U>>()))
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a worker finishes"))
            .collect()
    })
}

pub fn fill<U: Send>(slots: &mut [U], work: impl Fn(usize, &mut U) + Sync) {
    let size = share(slots.len());
    thread::scope(|scope| {
        for (part, chunk) in slots.chunks_mut(size).enumerate() {
            let work = &work;
            scope.spawn(move || {
                for (offset, slot) in chunk.iter_mut().enumerate() {
                    work(part * size + offset, slot);
                }
            });
        }
    });
}

fn share(count: usize) -> usize {
    let workers = thread::available_parallelism().map_or(1, NonZeroUsize::get);
    count.div_ceil(workers).max(1)
}

#[cfg(test)]
mod tests {
    use super::{fill, map};

    #[test]
    fn a_map_keeps_the_order_of_its_items() {
        let items: Vec<u32> = (0..10_000).collect();
        assert_eq!(
            map(&items, |item| item * 2),
            items.iter().map(|item| item * 2).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_fill_gives_each_slot_its_own_index() {
        let mut slots = vec![0_usize; 10_000];
        fill(&mut slots, |index, slot| *slot = index * 3);
        assert!(
            slots
                .iter()
                .enumerate()
                .all(|(index, &slot)| slot == index * 3)
        );
    }

    #[test]
    fn empty_work_is_done_at_once() {
        assert_eq!(map(&[] as &[u8], |item| *item), Vec::<u8>::new());
        fill(&mut [] as &mut [u8], |_, _| {});
    }
}
