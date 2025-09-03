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
            let result = self.u8pool.get(self.current);
            self.current += 1;
            Some(result)
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

/// Iterator over key-value pairs in a U8Pool
pub struct U8PoolPairIter<'a> {
    pub(crate) u8pool: &'a U8Pool<'a>,
    pub(crate) current_pair: usize,
}

impl<'a> Iterator for U8PoolPairIter<'a> {
    type Item = (&'a [u8], Option<&'a [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        let key_index = self.current_pair * 2;

        if key_index >= self.u8pool.len() {
            return None;
        }

        let key = self.u8pool.get(key_index);
        let value = if key_index + 1 < self.u8pool.len() {
            Some(self.u8pool.get(key_index + 1))
        } else {
            None
        };

        self.current_pair += 1;
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_pairs = if self.u8pool.is_empty() {
            0
        } else {
            self.u8pool.len().div_ceil(2) - self.current_pair
        };
        (remaining_pairs, Some(remaining_pairs))
    }
}

impl<'a> ExactSizeIterator for U8PoolPairIter<'a> {}
