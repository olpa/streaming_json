use crate::core::BufVec;

/// Iterator over slices in a BufVec
pub struct BufVecIter<'a> {
    bufvec: &'a BufVec<'a>,
    current: usize,
}

impl<'a> Iterator for BufVecIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.bufvec.len() {
            let result = self.bufvec.get(self.current);
            self.current += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.bufvec.len() - self.current;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for BufVecIter<'a> {}

impl<'a> IntoIterator for &'a BufVec<'a> {
    type Item = &'a [u8];
    type IntoIter = BufVecIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BufVecIter {
            bufvec: self,
            current: 0,
        }
    }
}

/// Iterator over key-value pairs in a BufVec
pub struct BufVecPairIter<'a> {
    pub(crate) bufvec: &'a BufVec<'a>,
    pub(crate) current_pair: usize,
}

impl<'a> Iterator for BufVecPairIter<'a> {
    type Item = (&'a [u8], Option<&'a [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        let key_index = self.current_pair * 2;

        if key_index >= self.bufvec.len() {
            return None;
        }

        let key = self.bufvec.get(key_index);
        let value = if key_index + 1 < self.bufvec.len() {
            Some(self.bufvec.get(key_index + 1))
        } else {
            None
        };

        self.current_pair += 1;
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_pairs = if self.bufvec.is_empty() {
            0
        } else {
            self.bufvec.len().div_ceil(2) - self.current_pair
        };
        (remaining_pairs, Some(remaining_pairs))
    }
}

impl<'a> ExactSizeIterator for BufVecPairIter<'a> {}
