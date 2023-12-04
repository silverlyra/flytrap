use std::num::NonZeroU32;

use rand::{thread_rng, Rng};

/// Generate [sequential][Seq] 32-bit ID’s.
///
/// To provide some level of collision resistance after a node restarts, bits
/// 20-28 of the initial value of a [new][Counter::new] counter will be randomly
/// flipped.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Counter(u32);

impl Counter {
    pub fn new() -> Self {
        Self(seed() | 1)
    }

    #[inline]
    pub fn next(&mut self) -> Seq {
        let (c, n) = (self.0, self.0.wrapping_add(1));
        self.0 = n;
        Seq::new(c)
    }
}

impl Iterator for Counter {
    type Item = Seq;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Counter::next(self))
    }
}

#[inline]
fn seed() -> u32 {
    let b: u8 = thread_rng().gen();
    (b as u32) << 20
}

/// A sequential [`u32`] ID.
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Seq(NonZeroU32);

impl Seq {
    #[inline]
    pub const fn new(value: u32) -> Self {
        match value {
            0 => panic!("seq values cannot be zero"),
            n => unsafe { Self(NonZeroU32::new_unchecked(n)) },
        }
    }

    #[inline]
    pub const fn receive(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    #[inline]
    pub const fn value(self) -> u32 {
        self.0.get()
    }

    #[inline]
    pub const fn write(value: &Option<Self>) -> u32 {
        match value {
            Some(seq) => seq.value(),
            None => 0,
        }
    }
}

impl From<Seq> for u32 {
    fn from(value: Seq) -> Self {
        value.value()
    }
}

#[cfg(test)]
impl From<u32> for Seq {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}
