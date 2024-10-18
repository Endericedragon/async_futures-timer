//! A simple binary heap with support for removal of arbitrary elements
//!
//! This heap is used to manage timer state in the event loop. All timeouts go
//! into this heap and we also cancel timeouts from this heap. The crucial
//! feature of this heap over the standard library's `BinaryHeap` is the ability
//! to remove arbitrary elements. (e.g. when a timer is canceled)
//!
//! Note that this heap is not at all optimized right now, it should hopefully
//! just work.


use core::mem;
use async_std::collections::Vec;

pub struct Heap<T> {
    // Binary heap of items, plus the slab index indicating what position in the
    // list they're in.
    items: Vec<(T, usize)>,

    // A map from a slab index (assigned to an item above) to the actual index
    // in the array the item appears at.
    index: Vec<SlabSlot<usize>>,
    next_index: usize,
}

enum SlabSlot<T> {
    Empty { next: usize },
    Full { value: T },
}

pub struct Slot {
    idx: usize,
}

impl<T: Ord> Heap<T> {
    pub fn new() -> Heap<T> {
        Heap {
            items: Vec::new(),
            index: Vec::new(),
            next_index: 0,
        }
    }

    /// Pushes an element onto this heap, returning a slot token indicating
    /// where it was pushed on to.
    ///
    /// The slot can later get passed to `remove` to remove the element from the
    /// heap, but only if the element was previously not removed from the heap.
    pub fn push(&mut self, t: T) -> Slot {
        self.assert_consistent();
        let len = self.items.len();
        let slot = SlabSlot::Full { value: len };
        let slot_idx = if self.next_index == self.index.len() {
            self.next_index += 1;
            self.index.push(slot);
            self.index.len() - 1
        } else {
            match mem::replace(&mut self.index[self.next_index], slot) {
                SlabSlot::Empty { next } => mem::replace(&mut self.next_index, next),
                SlabSlot::Full { .. } => panic!(),
            }
        };
        self.items.push((t, slot_idx));
        self.percolate_up(len);
        self.assert_consistent();
        Slot { idx: slot_idx }
    }

    pub fn peek(&self) -> Option<&T> {
        self.assert_consistent();
        self.items.first().map(|i| &i.0)
    }

    pub fn pop(&mut self) -> Option<T> {
        self.assert_consistent();
        if self.items.is_empty() {
            return None;
        }
        let slot = Slot {
            idx: self.items[0].1,
        };
        Some(self.remove(slot))
    }

    pub fn remove(&mut self, slot: Slot) -> T {
        self.assert_consistent();
        let empty = SlabSlot::Empty {
            next: self.next_index,
        };
        let idx = match mem::replace(&mut self.index[slot.idx], empty) {
            SlabSlot::Full { value } => value,
            SlabSlot::Empty { .. } => panic!(),
        };
        self.next_index = slot.idx;
        let (item, slot_idx) = self.items.swap_remove(idx);
        debug_assert_eq!(slot.idx, slot_idx);
        if idx < self.items.len() {
            set_index(&mut self.index, self.items[idx].1, idx);
            if self.items[idx].0 < item {
                self.percolate_up(idx);
            } else {
                self.percolate_down(idx);
            }
        }
        self.assert_consistent();
        item
    }

    fn percolate_up(&mut self, mut idx: usize) -> usize {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if self.items[idx].0 >= self.items[parent].0 {
                break;
            }
            let (a, b) = self.items.split_at_mut(idx);
            mem::swap(&mut a[parent], &mut b[0]);
            set_index(&mut self.index, a[parent].1, parent);
            set_index(&mut self.index, b[0].1, idx);
            idx = parent;
        }
        idx
    }

    fn percolate_down(&mut self, mut idx: usize) -> usize {
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;

            let mut swap_left = true;
            match (self.items.get(left), self.items.get(right)) {
                (Some(left), None) => {
                    if left.0 >= self.items[idx].0 {
                        break;
                    }
                }
                (Some(left), Some(right)) => {
                    if left.0 < self.items[idx].0 {
                        if right.0 < left.0 {
                            swap_left = false;
                        }
                    } else if right.0 < self.items[idx].0 {
                        swap_left = false;
                    } else {
                        break;
                    }
                }

                (None, None) => break,
                (None, Some(_right)) => panic!("not possible"),
            }

            let (a, b) = if swap_left {
                self.items.split_at_mut(left)
            } else {
                self.items.split_at_mut(right)
            };
            mem::swap(&mut a[idx], &mut b[0]);
            set_index(&mut self.index, a[idx].1, idx);
            set_index(&mut self.index, b[0].1, a.len());
            idx = a.len();
        }
        idx
    }

    fn assert_consistent(&self) {
        if !cfg!(assert_timer_heap_consistent) {
            return;
        }

        assert_eq!(
            self.items.len(),
            self.index
                .iter()
                .filter(|slot| {
                    match **slot {
                        SlabSlot::Full { .. } => true,
                        SlabSlot::Empty { .. } => false,
                    }
                })
                .count()
        );

        for (i, &(_, j)) in self.items.iter().enumerate() {
            let index = match self.index[j] {
                SlabSlot::Full { value } => value,
                SlabSlot::Empty { .. } => panic!(),
            };
            if index != i {
                panic!(
                    "self.index[j] != i : i={} j={} self.index[j]={}",
                    i, j, index
                );
            }
        }

        for (i, (item, _)) in self.items.iter().enumerate() {
            if i > 0 {
                assert!(*item >= self.items[(i - 1) / 2].0, "bad at index: {}", i);
            }
            if let Some(left) = self.items.get(2 * i + 1) {
                assert!(*item <= left.0, "bad left at index: {}", i);
            }
            if let Some(right) = self.items.get(2 * i + 2) {
                assert!(*item <= right.0, "bad right at index: {}", i);
            }
        }
    }
}

fn set_index<T>(slab: &mut Vec<SlabSlot<T>>, slab_slot: usize, val: T) {
    match slab[slab_slot] {
        SlabSlot::Full { ref mut value } => *value = val,
        SlabSlot::Empty { .. } => panic!(),
    }
}
