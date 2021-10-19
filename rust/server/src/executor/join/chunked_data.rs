use crate::error::Result;
use std::ops::{Index, IndexMut};

/// Id of one row in chunked data.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub(super) struct RowId {
    chunk_id: u32,
    row_id: u32,
}

/// [`ChunkedData`] is in fact a list of list.
///
/// We use this data structure instead of [`Vec<Vec<V>>`] to save allocation call.
#[derive(Default)]
pub(super) struct ChunkedData<V> {
    data: Vec<V>,
    chunk_offsets: Vec<usize>,
}

pub(super) struct AllRowIdIter<'a> {
    cur: RowId,
    chunk_offsets: &'a [usize],
}

impl<'a> Iterator for AllRowIdIter<'a> {
    type Item = RowId;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.cur.chunk_id() + 1) >= self.chunk_offsets.len() {
            None
        } else {
            let ret = Some(self.cur);
            let current_chunk_row_count = self.chunk_offsets[self.cur.chunk_id() + 1]
                - self.chunk_offsets[self.cur.chunk_id()];
            self.cur = self.cur.next_row(current_chunk_row_count);
            ret
        }
    }
}

impl RowId {
    pub(super) fn new(chunk_id: usize, row_id: usize) -> Self {
        Self {
            chunk_id: chunk_id as u32,
            row_id: row_id as u32,
        }
    }

    #[inline(always)]
    pub(super) fn chunk_id(&self) -> usize {
        self.chunk_id as usize
    }

    #[inline(always)]
    pub(super) fn row_id(&self) -> usize {
        self.row_id as usize
    }

    #[inline(always)]
    pub(super) fn next_row(self, cur_chunk_row_count: usize) -> RowId {
        if (self.row_id + 1) >= (cur_chunk_row_count as u32) {
            RowId {
                chunk_id: self.chunk_id + 1,
                row_id: 0,
            }
        } else {
            RowId {
                chunk_id: self.chunk_id,
                row_id: self.row_id + 1,
            }
        }
    }
}

impl<V> ChunkedData<V> {
    pub(super) fn with_chunk_sizes<C>(chunk_sizes: C) -> Result<Self>
    where
        C: IntoIterator<Item = usize>,
        V: Default,
    {
        let chunk_sizes = chunk_sizes.into_iter();
        let mut chunk_offsets = Vec::with_capacity(chunk_sizes.size_hint().0 + 1);
        let mut cur = 0usize;
        chunk_offsets.push(0);
        for chunk_size in chunk_sizes {
            ensure!(chunk_size > 0, "Chunk size can't be zero!");
            cur += chunk_size;
            chunk_offsets.push(cur);
        }

        let mut data = Vec::with_capacity(cur);
        data.resize_with(cur, V::default);

        Ok(Self {
            data,
            chunk_offsets,
        })
    }

    fn index_in_data(&self, index: RowId) -> usize {
        self.chunk_offsets[index.chunk_id()] + index.row_id()
    }

    pub(super) fn all_row_ids(&self) -> impl Iterator<Item = RowId> + '_ {
        AllRowIdIter {
            cur: RowId::default(),
            chunk_offsets: &self.chunk_offsets,
        }
    }
}

impl<V> Index<RowId> for ChunkedData<V> {
    type Output = V;

    fn index(&self, index: RowId) -> &V {
        &self.data[self.index_in_data(index)]
    }
}

impl<V> IndexMut<RowId> for ChunkedData<V> {
    fn index_mut(&mut self, index: RowId) -> &mut V {
        let index_in_data = self.index_in_data(index);
        &mut self.data[index_in_data]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_row_ids() {
        let chunk_sizes = vec![4, 3, 1, 2usize];

        let chunked_data =
            ChunkedData::<()>::with_chunk_sizes(chunk_sizes).expect("Build chunked data.");
        let expected_all_row_ids = vec![
            RowId::new(0, 0),
            RowId::new(0, 1),
            RowId::new(0, 2),
            RowId::new(0, 3),
            RowId::new(1, 0),
            RowId::new(1, 1),
            RowId::new(1, 2),
            RowId::new(2, 0),
            RowId::new(3, 0),
            RowId::new(3, 1),
        ];

        assert_eq!(
            expected_all_row_ids,
            chunked_data.all_row_ids().collect::<Vec<RowId>>()
        );
    }

    #[test]
    fn test_indexes() {
        let chunk_sizes = vec![4, 3, 1, 2usize];

        let mut chunked_data =
            ChunkedData::<usize>::with_chunk_sizes(chunk_sizes).expect("Build chunked data.");

        let row_ids = vec![
            RowId::new(0, 3),
            RowId::new(1, 1),
            RowId::new(2, 0),
            RowId::new(3, 1),
        ];

        for row_id in &row_ids {
            chunked_data[*row_id] = row_id.chunk_id() + row_id.row_id();
        }

        for row_id in &row_ids {
            let expected = row_id.chunk_id() + row_id.row_id();
            assert_eq!(expected, chunked_data[*row_id]);
        }
    }

    #[test]
    #[should_panic]
    fn test_zero_chunk_size_should_fail() {
        let chunk_sizes = vec![4, 3, 0, 1, 2usize];
        ChunkedData::<()>::with_chunk_sizes(chunk_sizes).unwrap();
    }
}
