use crate::NGRAM_MAX_SIZE;
use ::core::{hash::BuildHasher, ops::RangeInclusive};
use ::std::collections::HashSet;
use alphabet_detector::ScriptLanguage;

#[derive(Debug)]
pub struct DetectorConfig<H: BuildHasher + Default> {
    pub languages: HashSet<ScriptLanguage, H>,
    pub(super) long_text_minlen: usize,
    pub(super) long_text_ngrams: RangeInclusive<usize>,
    pub(super) short_text_ngrams: RangeInclusive<usize>,
}

impl<H: BuildHasher + Default> Default for DetectorConfig<H> {
    #[inline]
    fn default() -> Self {
        Self {
            languages: HashSet::with_hasher(H::default()),
            long_text_minlen: 120,
            long_text_ngrams: 3..=NGRAM_MAX_SIZE,
            short_text_ngrams: 1..=NGRAM_MAX_SIZE,
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
        self.long_text_ngrams = 3..=3;
        self.short_text_ngrams = 1..=3;
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
    pub fn long_ngrams(mut self, long_text_ngrams: RangeInclusive<usize>) -> Self {
        debug_assert!(*long_text_ngrams.start() > 0 && *long_text_ngrams.end() <= NGRAM_MAX_SIZE);
        self.long_text_ngrams = long_text_ngrams;
        self
    }

    #[inline]
    pub fn short_ngrams(mut self, short_text_ngrams: RangeInclusive<usize>) -> Self {
        debug_assert!(*short_text_ngrams.start() > 0 && *short_text_ngrams.end() <= NGRAM_MAX_SIZE);
        self.short_text_ngrams = short_text_ngrams;
        self
    }

    #[inline]
    pub fn minlen(mut self, long_text_minlen: usize) -> Self {
        self.long_text_minlen = long_text_minlen;
        self
    }
}
