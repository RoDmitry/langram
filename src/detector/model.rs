use crate::{
    ngram_size::{NgramSize, NGRAM_MAX_LEN},
    ngrams::NgramString,
};
use compact_str::CompactString;
use debug_unsafe::{arraystring::ArrayStringFrom, slice::SliceGetter};
use rustc_hash::FxHashMap;

pub(crate) type ModelNgrams<Ngram> = FxHashMap<Ngram, f64>;
type ModelNgramsArr = [ModelNgrams<NgramString>; NGRAM_MAX_LEN];

pub(crate) trait NgramFromChars: Sized {
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self;
}

impl NgramFromChars for NgramString {
    #[inline(always)]
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self {
        Self::from_chars_safe_unchecked(chars)
    }
}

impl NgramFromChars for CompactString {
    #[inline(always)]
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self {
        chars.into_iter().collect()
    }
}

pub(super) struct Model {
    pub(super) ngrams: ModelNgramsArr,
    pub(super) ngram_min_probability: f64,
    pub(super) wordgrams: ModelNgrams<CompactString>,
    pub(super) wordgram_min_probability: f64,
}

impl Default for Model {
    #[inline]
    fn default() -> Self {
        Self {
            ngrams: Default::default(),
            wordgrams: Default::default(),
            ngram_min_probability: f64::NEG_INFINITY,
            wordgram_min_probability: f64::NEG_INFINITY,
        }
    }
}

impl Model {
    fn count_min_probability<Ngram>(model_ngrams: &ModelNgrams<Ngram>) -> f64 {
        if !model_ngrams.is_empty() {
            (1.0 / model_ngrams.len() as f64).ln()
        } else {
            f64::NEG_INFINITY
        }
    }

    #[inline]
    pub(super) fn update_ngrams(
        &mut self,
        model_ngrams: ModelNgrams<NgramString>,
        ngram_size: NgramSize,
    ) {
        if matches!(ngram_size, NgramSize::Uni) {
            self.ngram_min_probability = Self::count_min_probability(&model_ngrams);
        }
        *self.ngrams.get_safe_unchecked_mut(ngram_size as usize) = model_ngrams;
    }

    #[inline]
    pub(super) fn update_wordgrams(&mut self, model_wordgrams: ModelNgrams<CompactString>) {
        self.wordgram_min_probability = Self::count_min_probability(&model_wordgrams);
        self.wordgrams = model_wordgrams;
    }

    #[cfg(test)]
    #[inline]
    pub(super) fn new(ngrams: ModelNgramsArr, wordgrams: ModelNgrams<CompactString>) -> Self {
        let ngram_min_probability =
            Self::count_min_probability(ngrams.get_safe_unchecked(NgramSize::Uni as usize));
        let wordgram_min_probability = Self::count_min_probability(&wordgrams);

        Self {
            ngrams,
            ngram_min_probability,
            wordgrams,
            wordgram_min_probability,
        }
    }
}
