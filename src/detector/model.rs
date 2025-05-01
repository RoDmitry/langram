use crate::ngrams::NgramString;
use compact_str::CompactString;
use debug_unsafe::slice::SliceGetter;
use rustc_hash::FxHashMap;

pub(crate) const NGRAM_MAX_LEN: usize = 5;
pub(crate) const NGRAMS_TOTAL_SIZE: usize = 6;

pub(crate) type ModelNgrams<Ngram> = FxHashMap<Ngram, f64>;
type ModelNgramsArr = [ModelNgrams<NgramString>; NGRAM_MAX_LEN];

pub(crate) trait NgramFromChars: Sized {
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self;
}

impl NgramFromChars for NgramString {
    #[inline(always)]
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self {
        Self::try_from_chars(chars).unwrap()
    }
}

impl NgramFromChars for CompactString {
    #[inline(always)]
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self {
        chars.into_iter().collect()
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub(crate) enum NgramsSize {
    Uni = 0,
    Bi = 1,
    Tri = 2,
    Quadri = 3,
    Five = 4,
    Word = 5,
}

impl From<usize> for NgramsSize {
    #[inline(always)]
    fn from(v: usize) -> Self {
        debug_assert!(
            (0..NGRAMS_TOTAL_SIZE).contains(&v),
            "NgramsSize {} is not in range 0..{NGRAMS_TOTAL_SIZE}",
            v
        );

        unsafe { core::mem::transmute(v) }
    }
}

impl NgramsSize {
    #[inline]
    pub(crate) fn into_file_name(self) -> &'static str {
        match self {
            Self::Uni => "unigrams.encom.br",
            Self::Bi => "bigrams.encom.br",
            Self::Tri => "trigrams.encom.br",
            Self::Quadri => "quadrigrams.encom.br",
            Self::Five => "fivegrams.encom.br",
            Self::Word => "wordgrams.encom.br",
        }
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
        ngrams_size: NgramsSize,
    ) {
        if matches!(ngrams_size, NgramsSize::Uni) {
            self.ngram_min_probability = Self::count_min_probability(&model_ngrams);
        }
        *self.ngrams.get_safe_unchecked_mut(ngrams_size as usize) = model_ngrams;
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
            Self::count_min_probability(ngrams.get_safe_unchecked(NgramsSize::Uni as usize));
        let wordgram_min_probability = Self::count_min_probability(&wordgrams);

        Self {
            ngrams,
            ngram_min_probability,
            wordgrams,
            wordgram_min_probability,
        }
    }
}
