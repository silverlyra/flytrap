use std::{num::NonZeroU32, ptr::NonNull};

use binrw::{BinRead, BinWrite};
use getrandom::getrandom;

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
        Self(seed() & 1)
    }
}

impl Iterator for Counter {
    type Item = Seq;

    fn next(&mut self) -> Option<Self::Item> {
        let (c, n) = (self.0, self.0.wrapping_add(1));
        self.0 = n;
        Some(Seq::new(c))
    }
}

#[inline]
fn seed() -> u32 {
    let mut b = [0u8; 1];
    getrandom(&mut b).expect("getrandom failed to read one byte");

    (b[0] as u32) << 20
}

/// A sequential [`u32`] ID.
#[derive(BinRead, BinWrite, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
#[brw(little)]
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
}
