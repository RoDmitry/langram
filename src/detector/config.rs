use crate::NGRAM_MAX_SIZE;
use ::core::{hash::BuildHasher, ops::RangeInclusive};
use ::std::collections::HashSet;
use alphabet_detector::ScriptLanguage;

pub struct DetectorConfig<S: BuildHasher + Default> {
    pub languages: HashSet<ScriptLanguage, S>,
    pub(super) long_text_minlen: usize,
    pub(super) long_text_ngrams: RangeInclusive<usize>,
    pub(super) short_text_ngrams: RangeInclusive<usize>,
}

impl<S: BuildHasher + Default> Default for DetectorConfig<S> {
    #[inline]
    fn default() -> Self {
        Self {
            languages: HashSet::with_hasher(S::default()),
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

impl<S: BuildHasher + Default> DetectorConfig<S> {
    #[inline]
    pub fn with_languages(languages: HashSet<ScriptLanguage, S>) -> Self {
        Self {
            languages,
            ..Default::default()
        }
    }

    #[inline]
    pub fn copy_with_languages(&self, languages: HashSet<ScriptLanguage, S>) -> Self {
        Self {
            languages,
            long_text_minlen: self.long_text_minlen,
            long_text_ngrams: self.long_text_ngrams.clone(),
            short_text_ngrams: self.short_text_ngrams.clone(),
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
    pub fn languages(mut self, languages: HashSet<ScriptLanguage, S>) -> Self {
        self.languages = languages;
        self
    }

    #[inline]
    pub fn all_languages(self) -> Self {
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
