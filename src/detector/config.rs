use super::NgramSize;
use ::core::hash::BuildHasher;
use ::std::collections::HashSet;
use alphabet_detector::ScriptLanguage;
use arrayvec::ArrayVec;
use strum::EnumCount;

pub(super) type TextNgramSizes = ArrayVec<NgramSize, { NgramSize::COUNT }>;

pub(super) trait TextNgramSizesTrait: Sized {
    fn merge(&mut self, ngram_sizes: impl Iterator<Item = NgramSize>);
    fn new_merged(ngram_sizes: impl Iterator<Item = NgramSize>) -> Self;
}

impl TextNgramSizesTrait for TextNgramSizes {
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

#[derive(Debug)]
pub struct DetectorConfig<H: BuildHasher + Default> {
    pub languages: HashSet<ScriptLanguage, H>,
    pub(super) long_text_minlen: usize,
    pub(super) long_text_ngrams: TextNgramSizes,
    pub(super) short_text_ngrams: TextNgramSizes,
}

impl<H: BuildHasher + Default> Default for DetectorConfig<H> {
    #[inline]
    fn default() -> Self {
        Self {
            languages: HashSet::with_hasher(H::default()),
            long_text_minlen: 120,
            long_text_ngrams: TextNgramSizes::new_merged(
                [
                    NgramSize::Tri,
                    NgramSize::Quadri,
                    NgramSize::Five,
                    NgramSize::Word,
                ]
                .into_iter(),
            ),
            short_text_ngrams: TextNgramSizes::new_merged(
                [
                    NgramSize::Uni,
                    NgramSize::Bi,
                    NgramSize::Tri,
                    NgramSize::Quadri,
                    NgramSize::Five,
                    NgramSize::Word,
                ]
                .into_iter(),
            ),
        }
    }
}

impl DetectorConfig<ahash::RandomState> {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Faster, but lower accuracy
    #[inline]
    pub fn new_max_trigrams() -> Self {
        Self::new().max_trigrams()
    }

    #[inline]
    pub fn new_all_languages() -> Self {
        Self::new().all_languages()
    }
}

impl<H: BuildHasher + Default> DetectorConfig<H> {
    #[inline]
    pub fn with_languages(languages: HashSet<ScriptLanguage, H>) -> Self {
        Self {
            languages,
            ..Default::default()
        }
    }

    /// Faster, but lower accuracy
    #[inline]
    pub fn max_trigrams(mut self) -> Self {
        self.long_text_ngrams =
            TextNgramSizes::new_merged([NgramSize::Tri, NgramSize::Word].into_iter());
        self.short_text_ngrams = TextNgramSizes::new_merged(
            [
                NgramSize::Uni,
                NgramSize::Bi,
                NgramSize::Tri,
                NgramSize::Word,
            ]
            .into_iter(),
        );
        self
    }

    #[inline]
    pub fn languages<H2: BuildHasher + Default>(
        self,
        languages: HashSet<ScriptLanguage, H2>,
    ) -> DetectorConfig<H2> {
        DetectorConfig {
            languages,
            long_text_minlen: self.long_text_minlen,
            long_text_ngrams: self.long_text_ngrams,
            short_text_ngrams: self.short_text_ngrams,
        }
    }

    #[inline]
    pub fn copy_with_languages<H2: BuildHasher + Default>(
        &self,
        languages: HashSet<ScriptLanguage, H2>,
    ) -> DetectorConfig<H2> {
        DetectorConfig {
            languages,
            long_text_minlen: self.long_text_minlen,
            long_text_ngrams: self.long_text_ngrams.clone(),
            short_text_ngrams: self.short_text_ngrams.clone(),
        }
    }

    #[inline]
    pub fn all_languages(self) -> DetectorConfig<ahash::RandomState> {
        self.languages(ScriptLanguage::all().collect())
    }

    #[inline]
    pub fn long_ngrams(mut self, ngrams: impl Iterator<Item = NgramSize>) -> Self {
        self.long_text_ngrams = TextNgramSizes::new_merged(ngrams);
        self
    }

    #[inline]
    pub fn short_ngrams(mut self, ngrams: impl Iterator<Item = NgramSize>) -> Self {
        self.short_text_ngrams = TextNgramSizes::new_merged(ngrams);
        self
    }

    #[inline]
    pub fn minlen(mut self, long_text_minlen: usize) -> Self {
        self.long_text_minlen = long_text_minlen;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{TextNgramSizes, TextNgramSizesTrait};
    use crate::detector::NgramSize;

    #[test]
    fn test_ngram_sizes_merge() {
        let mut ngrams = TextNgramSizes::new_merged([NgramSize::Tri, NgramSize::Bi].into_iter());
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
