use crate::{ngrams::NgramString, NGRAM_MAX_SIZE};
use debug_unsafe::slice::SliceGetter;
use rustc_hash::FxHashMap;

pub(crate) type ModelNgrams = FxHashMap<NgramString, f64>;
type ModelNgramsArr = [ModelNgrams; NGRAM_MAX_SIZE];

pub(super) struct Model {
    pub(super) ngrams: ModelNgramsArr,
    pub(super) min_probability: f64,
}

impl Default for Model {
    #[inline]
    fn default() -> Self {
        Self {
            ngrams: Default::default(),
            min_probability: f64::NEG_INFINITY,
        }
    }
}

impl Model {
    #[inline]
    pub(super) fn update_ngram(&mut self, model_ngrams: ModelNgrams, index: usize) {
        if index == 0 {
            self.min_probability = if !model_ngrams.is_empty() {
                (1.0 / model_ngrams.len() as f64).ln()
            } else {
                f64::NEG_INFINITY
            }
        }
        if let Some(n) = self.ngrams.get_mut(index) {
            *n = model_ngrams
        }
    }
}

impl From<ModelNgramsArr> for Model {
    #[inline]
    fn from(ngrams: ModelNgramsArr) -> Self {
        let min_probability = if !ngrams.get_safe_unchecked(0).is_empty() {
            (1.0 / ngrams.get_safe_unchecked(0).len() as f64).ln()
        } else {
            f64::NEG_INFINITY
        };

        Self {
            ngrams,
            min_probability,
        }
    }
}
