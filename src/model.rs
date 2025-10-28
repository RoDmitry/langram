use crate::ngram_size::NgramSize;
use ::std::collections::HashMap;
#[cfg(debug_assertions)]
use debug_unsafe::slice::SliceGetter;
use strum::EnumCount;

// pub type ModelNgrams = entropy_map::Map<String, f64, 64, 10, u16, rustc_hash::FxHasher>;
pub type ModelNgrams = HashMap<String, f64, rustc_hash::FxBuildHasher>;
type ModelNgramsArr = [ModelNgrams; NgramSize::COUNT];

/* pub(crate) trait NgramFromChars: Sized {
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self;
}

impl NgramFromChars for String {
    #[inline(always)]
    fn from_chars(chars: impl IntoIterator<Item = char>) -> Self {
        chars.into_iter().collect()
    }
} */

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct Model {
    pub ngrams: ModelNgramsArr,
    pub ngram_min_probability: f64,
}

impl Default for Model {
    #[inline]
    fn default() -> Self {
        Self {
            ngrams: Default::default(),
            ngram_min_probability: f64::NEG_INFINITY,
        }
    }
}

impl Model {
    #[inline]
    pub fn compute_min_probability(size: usize) -> f64 {
        (1.0 / (size as f64)).ln()
    }

    #[cfg(debug_assertions)]
    #[inline]
    pub fn new_mock(ngrams: ModelNgramsArr) -> Self {
        let ngram_min_probability =
            Self::compute_min_probability(ngrams.get_safe_unchecked(NgramSize::Uni as usize).len());

        Self {
            ngrams,
            ngram_min_probability,
        }
    }
}
