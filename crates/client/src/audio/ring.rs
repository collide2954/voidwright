use std::array::from_fn;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub const WORDS: usize = 8;

struct Shared {
    slots: Box<[[AtomicU32; WORDS]]>,
    written: AtomicUsize,
    read: AtomicUsize,
}

pub struct Writer {
    shared: Arc<Shared>,
}

pub struct Reader {
    shared: Arc<Shared>,
}

#[must_use]
pub fn ring(capacity: usize) -> (Writer, Reader) {
    let shared = Arc::new(Shared {
        slots: (0..capacity.max(1))
            .map(|_| from_fn(|_| AtomicU32::new(0)))
            .collect(),
        written: AtomicUsize::new(0),
        read: AtomicUsize::new(0),
    });
    (
        Writer {
            shared: Arc::clone(&shared),
        },
        Reader { shared },
    )
}

impl Writer {
    pub fn push(&mut self, words: [u32; WORDS]) -> bool {
        let shared = &*self.shared;
        let written = shared.written.load(Ordering::Relaxed);
        let read = shared.read.load(Ordering::Acquire);
        if written.wrapping_sub(read) >= shared.slots.len() {
            return false;
        }
        let slot = &shared.slots[written % shared.slots.len()];
        for (cell, word) in slot.iter().zip(words) {
            cell.store(word, Ordering::Relaxed);
        }
        shared
            .written
            .store(written.wrapping_add(1), Ordering::Release);
        true
    }
}

impl Reader {
    pub fn pop(&mut self) -> Option<[u32; WORDS]> {
        let shared = &*self.shared;
        let read = shared.read.load(Ordering::Relaxed);
        if read == shared.written.load(Ordering::Acquire) {
            return None;
        }
        let slot = &shared.slots[read % shared.slots.len()];
        let words = from_fn(|index| slot[index].load(Ordering::Relaxed));
        shared.read.store(read.wrapping_add(1), Ordering::Release);
        Some(words)
    }
}

#[cfg(test)]
mod tests {
    use super::{WORDS, ring};
    use std::thread;

    fn words(value: u32) -> [u32; WORDS] {
        [value; WORDS]
    }

    #[test]
    fn commands_come_out_in_the_order_they_went_in() {
        let (mut writer, mut reader) = ring(4);
        for value in 1..=3 {
            assert!(writer.push(words(value)));
        }
        for value in 1..=3 {
            assert_eq!(reader.pop(), Some(words(value)));
        }
        assert_eq!(reader.pop(), None);
    }

    #[test]
    fn a_full_ring_refuses_more_until_the_reader_catches_up() {
        let (mut writer, mut reader) = ring(2);
        assert!(writer.push(words(1)));
        assert!(writer.push(words(2)));
        assert!(!writer.push(words(3)));
        assert_eq!(reader.pop(), Some(words(1)));
        assert!(writer.push(words(3)));
        assert_eq!(reader.pop(), Some(words(2)));
        assert_eq!(reader.pop(), Some(words(3)));
    }

    #[test]
    fn a_writer_thread_and_a_reader_thread_pass_every_command_in_order() {
        const COUNT: u32 = 20_000;
        let (mut writer, mut reader) = ring(16);
        let sender = thread::spawn(move || {
            for value in 0..COUNT {
                let packed = std::array::from_fn(|index| value ^ u32::try_from(index).unwrap());
                while !writer.push(packed) {
                    thread::yield_now();
                }
            }
        });
        let mut expected = 0;
        while expected < COUNT {
            if let Some(packed) = reader.pop() {
                let check = std::array::from_fn(|index| expected ^ u32::try_from(index).unwrap());
                assert_eq!(packed, check);
                expected += 1;
            } else {
                thread::yield_now();
            }
        }
        sender.join().unwrap();
        assert_eq!(reader.pop(), None);
    }
}
