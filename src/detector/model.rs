use crate::ngrams::NgramString;
use debug_unsafe::slice::SliceGetter;
use rustc_hash::FxHashMap;

pub(crate) const NGRAM_MAX_LEN: usize = 5;
pub(crate) const NGRAMS_TOTAL_SIZE: usize = 5;

pub(crate) type ModelNgrams = FxHashMap<NgramString, f64>;
type ModelNgramsArr = [ModelNgrams; NGRAMS_TOTAL_SIZE];

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub(crate) enum NgramsSize {
    Uni = 0,
    Bi = 1,
    Tri = 2,
    Quadri = 3,
    Five = 4,
    // Word = 5,
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
        }
    }
}

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
    pub(super) fn update_ngrams(
        &mut self,
        model_ngrams: ModelNgrams<NgramString>,
        ngrams_size: NgramsSize,
    ) {
        if matches!(ngrams_size, NgramsSize::Uni) {
            self.min_probability = if !model_ngrams.is_empty() {
                (1.0 / model_ngrams.len() as f64).ln()
            } else {
                f64::NEG_INFINITY
            }
        }
        *self.ngrams.get_safe_unchecked_mut(ngrams_size as usize) = model_ngrams;
    }
}

#[cfg(test)]
impl From<ModelNgramsArr> for Model {
    #[inline]
    fn from(ngrams: ModelNgramsArr) -> Self {
        let min_probability = if !ngrams
            .get_safe_unchecked(NgramsSize::Uni as usize)
            .is_empty()
        {
            (1.0 / ngrams.get_safe_unchecked(NgramsSize::Uni as usize).len() as f64).ln()
        } else {
            f64::NEG_INFINITY
        };

        Self {
            ngrams,
            min_probability,
        }
    }
}
