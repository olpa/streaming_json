use crate::core::U8Pool;
use crate::slice_descriptor::{SliceDescriptorIter, SliceDescriptorRevIter};

/// Iterator over slices in a `U8Pool`
pub struct U8PoolIter<'a> {
    data: &'a [u8],
    descriptor_iter: SliceDescriptorIter<'a>,
}

impl<'a> Iterator for U8PoolIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let (start, length) = self.descriptor_iter.next()?;
        self.data.get(start..start + length)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.descriptor_iter.size_hint()
    }
}

impl ExactSizeIterator for U8PoolIter<'_> {}

impl<'a> IntoIterator for &'a U8Pool<'a> {
    type Item = &'a [u8];
    type IntoIter = U8PoolIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        U8PoolIter {
            data: self.data(),
            descriptor_iter: self.descriptor_iter(),
        }
    }
}

/// Reverse iterator over slices in a `U8Pool`
pub struct U8PoolRevIter<'a> {
    data: &'a [u8],
    descriptor_iter: SliceDescriptorRevIter<'a>,
}

impl<'a> U8PoolRevIter<'a> {
    pub(crate) fn new(u8pool: &'a U8Pool<'a>) -> Self {
        Self {
            data: u8pool.data(),
            descriptor_iter: u8pool.descriptor_iter_rev(),
        }
    }
}

impl<'a> Iterator for U8PoolRevIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let (start, length) = self.descriptor_iter.next()?;
        self.data.get(start..start + length)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.descriptor_iter.size_hint()
    }
}

impl ExactSizeIterator for U8PoolRevIter<'_> {}

/// Iterator over key-value pairs in a `U8Pool`
pub struct U8PoolPairIter<'a> {
    iter: U8PoolIter<'a>,
}

impl<'a> U8PoolPairIter<'a> {
    pub(crate) fn new(u8pool: &'a U8Pool<'a>) -> Self {
        Self {
            iter: u8pool.iter(),
        }
    }
}

impl<'a> Iterator for U8PoolPairIter<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.iter.next()?;
        let value = self.iter.next()?;
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();
        // Each pair consumes exactly 2 items, so divide by 2 (ignore incomplete pairs)
        let pairs_lower = lower / 2;
        let pairs_upper = upper.map(|u| u / 2);
        (pairs_lower, pairs_upper)
    }
}

impl ExactSizeIterator for U8PoolPairIter<'_> {}
