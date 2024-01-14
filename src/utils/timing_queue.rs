use std::ops::{Add, Sub};

#[derive(Debug, Clone)]
pub struct TimingQueue<T> {
    next_key: i32,
    /// sorted in ascending order according to their timing
    entries: Vec<Entry<T>>,
}

#[derive(Debug, Clone)]
struct Entry<T> {
    timing: Timing,
    key: EntryKey,
    element: T,
}

impl<T> Default for TimingQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TimingQueue<T> {
    pub fn new() -> Self {
        TimingQueue {
            next_key: 0,
            entries: vec![],
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|e| &e.element)
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entries.iter_mut().map(|e| &mut e.element)
    }

    pub fn insert(&mut self, element: T, timing: Timing) -> EntryKey {
        let key = EntryKey(self.next_key);
        self.next_key += 1;

        let entry = Entry {
            key,
            element,
            timing,
        };
        let insertion_index = self.entries.iter().enumerate().find_map(|(i, e)| {
            if e.timing > timing {
                Some(i)
            } else {
                None
            }
        });
        match insertion_index {
            Some(i) => self.entries.insert(i, entry),
            None => self.entries.push(entry),
        }

        key
    }

    pub fn remove(&mut self, key: EntryKey) -> Option<T> {
        let index =
            self.entries
                .iter()
                .enumerate()
                .find_map(|(i, e)| if e.key == key { Some(i) } else { None })?;
        let element = self.entries.remove(index).element;
        Some(element)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryKey(i32);

/// Timing can be thought of as the inverse of Priority.
/// A high timing value means, a function will be executed later in a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timing(i32);

impl Timing {
    pub const VERY_EARLY: Timing = Timing(-200000);
    pub const EARLY: Timing = Timing(-100000);
    pub const DEFAULT: Timing = Timing(0);
    pub const LATE: Timing = Timing(100000);
    pub const VERY_LATE: Timing = Timing(200000);
}

impl Default for Timing {
    fn default() -> Self {
        Timing::DEFAULT
    }
}

impl Add<i32> for Timing {
    type Output = Timing;

    fn add(self, rhs: i32) -> Self::Output {
        Timing(self.0 + rhs)
    }
}

impl Sub<i32> for Timing {
    type Output = Timing;

    fn sub(self, rhs: i32) -> Self::Output {
        Timing(self.0 - rhs)
    }
}
