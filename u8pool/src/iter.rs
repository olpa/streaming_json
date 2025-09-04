use crate::core::U8Pool;

/// Iterator over slices in a U8Pool
pub struct U8PoolIter<'a> {
    u8pool: &'a U8Pool<'a>,
    current: usize,
}

impl<'a> Iterator for U8PoolIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.u8pool.len() {
            // Direct access to avoid redundant bounds checks
            let (start, length) = self.u8pool.get_slice_descriptor(self.current);
            self.current += 1;
            // Safety: get_slice_descriptor returns valid bounds that were validated during push
            unsafe {
                Some(self.u8pool.buffer.get_unchecked(start..start + length))
            }
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.u8pool.len() - self.current;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for U8PoolIter<'a> {}

impl<'a> IntoIterator for &'a U8Pool<'a> {
    type Item = &'a [u8];
    type IntoIter = U8PoolIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        U8PoolIter {
            u8pool: self,
            current: 0,
        }
    }
}

/// Reverse iterator over slices in a U8Pool
pub struct U8PoolRevIter<'a> {
    u8pool: &'a U8Pool<'a>,
    current: usize,
}

impl<'a> U8PoolRevIter<'a> {
    pub(crate) fn new(u8pool: &'a U8Pool<'a>) -> Self {
        Self {
            u8pool,
            current: u8pool.len(),
        }
    }
}

impl<'a> Iterator for U8PoolRevIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > 0 {
            self.current -= 1;
            // Direct access to avoid redundant bounds checks
            let (start, length) = self.u8pool.get_slice_descriptor(self.current);
            // Safety: get_slice_descriptor returns valid bounds that were validated during push
            unsafe {
                Some(self.u8pool.buffer.get_unchecked(start..start + length))
            }
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.current, Some(self.current))
    }
}

impl<'a> ExactSizeIterator for U8PoolRevIter<'a> {}

/// Iterator over key-value pairs in a U8Pool
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

impl<'a> ExactSizeIterator for U8PoolPairIter<'a> {}
