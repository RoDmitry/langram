use arrayvec::ArrayVec;
use strum::EnumCount;
use strum_macros::{EnumCount, EnumIter};

pub(crate) const NGRAM_MAX_LEN: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EnumCount, EnumIter)]
#[repr(usize)]
pub enum NgramSize {
    Uni = 0,
    Bi = 1,
    Tri = 2,
    Quadri = 3,
    Five = 4,
    Word = 5,
}

impl From<usize> for NgramSize {
    #[inline(always)]
    fn from(v: usize) -> Self {
        debug_assert!(
            (0..NgramSize::COUNT).contains(&v),
            "NgramsSize {v} is not in range 0..{}",
            NgramSize::COUNT
        );

        unsafe { core::mem::transmute(v) }
    }
}

pub type NgramSizes = ArrayVec<NgramSize, { NgramSize::COUNT }>;

pub trait NgramSizesTrait: Sized {
    fn merge(&mut self, ngram_sizes: impl Iterator<Item = NgramSize>);
    fn new_merged(ngram_sizes: impl Iterator<Item = NgramSize>) -> Self;
}

impl NgramSizesTrait for NgramSizes {
    fn merge(&mut self, ngram_sizes: impl Iterator<Item = NgramSize>) {
        for ngram_size in ngram_sizes {
            if !self.contains(&ngram_size) {
                self.push(ngram_size);
            }
        }
        self.sort_unstable();
    }

    #[inline]
    fn new_merged(ngram_sizes: impl Iterator<Item = NgramSize>) -> Self {
        let mut new = Self::new_const();
        new.merge(ngram_sizes);
        new
    }
}

#[cfg(test)]
mod tests {
    use super::{NgramSize, NgramSizes, NgramSizesTrait};

    #[test]
    fn test_ngram_sizes_merge() {
        let mut ngrams = NgramSizes::new_merged([NgramSize::Tri, NgramSize::Bi].into_iter());
        ngrams.merge(
            [
                NgramSize::Five,
                NgramSize::Uni,
                NgramSize::Bi,
                NgramSize::Quadri,
                NgramSize::Word,
            ]
            .into_iter(),
        );

        assert_eq!(
            ngrams.as_slice(),
            &[
                NgramSize::Uni,
                NgramSize::Bi,
                NgramSize::Tri,
                NgramSize::Quadri,
                NgramSize::Five,
                NgramSize::Word,
            ]
        );
    }
}
