use crate::{model::Model, ngram_size::NGRAM_MAX_LEN, NgramSize};
use ::std::{collections::HashMap, fmt};
use alphabet_detector::{ScriptLanguage, ScriptLanguageArr};
use debug_unsafe::slice::SliceGetter;
use rkyv::util::AlignedVec;
use strum::IntoEnumIterator;

pub(crate) type StorageNgrams = HashMap<String, Vec<(u16, f64)>, rustc_hash::FxBuildHasher>;
// Vec because array requires 64-bit pointers, failed with
// "out of range integral type conversion attempted"
pub(crate) type StorageNgramsArr = Vec<StorageNgrams>;

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct BinStorage {
    pub(crate) langs_ngram_min_probability: ScriptLanguageArr<f64>,
    pub(crate) ngrams: StorageNgramsArr,
    pub(crate) wordgrams: StorageNgrams,
    pub(crate) wordgram_min_probability: f64,
    pub(crate) hash: u64,
}

impl Default for BinStorage {
    #[inline]
    fn default() -> Self {
        Self {
            langs_ngram_min_probability: ::core::array::from_fn(|_| f64::NEG_INFINITY),
            ngrams: vec![Default::default(); NGRAM_MAX_LEN],
            // can't be included in ngrams, requires 64-bit pointers
            wordgrams: Default::default(),
            wordgram_min_probability: Default::default(),
            hash: ScriptLanguage::HASH,
        }
    }
}

impl fmt::Debug for BinStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileStorage")
            .field(
                "langs_ngram_min_probability",
                &self.langs_ngram_min_probability,
            )
            .field("wordgram_min_probability", &self.wordgram_min_probability)
            .field("hash", &self.hash)
            .finish_non_exhaustive()
    }
}

#[inline]
fn compute_min_probability(size: usize) -> f64 {
    (1.0 / (size as f64)).ln()
}

impl BinStorage {
    pub const FILE_NAME: &str = "langram_models.bin";

    pub fn add(&mut self, lang: ScriptLanguage, mut model: Model) {
        let model_wordgrams =
            ::core::mem::take(model.get_safe_unchecked_mut(NgramSize::Word as usize));

        if !model_wordgrams.is_empty() {
            for (word, prob) in model_wordgrams {
                self.wordgram_min_probability = self.wordgram_min_probability.min(prob * 4.0);
                let entry = self.wordgrams.entry(word).or_default();
                entry.push((lang as u16, prob));
            }
        }

        for (ngram_size, model_ngrams) in model.into_iter().enumerate() {
            let ngram_size = NgramSize::from(ngram_size);
            if ngram_size == NgramSize::Word {
                continue;
            }
            if ngram_size == NgramSize::Uni {
                *self
                    .langs_ngram_min_probability
                    .get_safe_unchecked_mut(lang as usize) =
                    compute_min_probability(model_ngrams.len());
            }

            let ngram_model = self.ngrams.get_safe_unchecked_mut(ngram_size as usize);
            for (word, prob) in model_ngrams {
                let entry = ngram_model.entry(word).or_default();
                entry.push((lang as u16, prob));
            }
        }
    }

    pub fn finalize(&mut self) {
        // reorder
        self.ngrams
            .iter_mut()
            .chain([&mut self.wordgrams])
            .inspect(|v| println!("len {:?}", v.len()))
            .flat_map(|v| v.iter_mut())
            .for_each(|(_, v)| {
                v.sort_by(|(l1, _), (l2, _)| {
                    ScriptLanguage::transmute_from_usize(*l1 as usize)
                        .cmp(&ScriptLanguage::transmute_from_usize(*l2 as usize))
                })
            });

        // normalize
        let max_prob = ScriptLanguage::iter().fold(f64::NEG_INFINITY, |acc, lang| {
            self.langs_ngram_min_probability
                .get_safe_unchecked(lang as usize)
                .max(acc)
        }) + 0.05;
        for lang in ScriptLanguage::iter() {
            *self
                .langs_ngram_min_probability
                .get_safe_unchecked_mut(lang as usize) -= max_prob;
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Result<AlignedVec, rkyv::rancor::Error> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
    }
}
