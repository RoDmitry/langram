use crate::{Model, NgramSize};
use ::std::{collections::HashMap, fmt};
use alphabet_detector::{ScriptLanguage, ScriptLanguageArr};
use debug_unsafe::slice::SliceGetter;
use rkyv::util::AlignedVec;

pub type StorageNgrams = HashMap<String, Vec<(u16, f64)>, rustc_hash::FxBuildHasher>;

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct BinStorage {
    pub langs_ngram_min_probability: ScriptLanguageArr<f64>,
    pub ngrams: StorageNgrams,
    pub wordgrams: StorageNgrams,
    pub wordgram_min_probability: f64,
    pub hash: u64,
}

impl Default for BinStorage {
    #[inline]
    fn default() -> Self {
        Self {
            langs_ngram_min_probability: ::core::array::from_fn(|_| f64::NEG_INFINITY),
            ngrams: Default::default(),
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
            .finish_non_exhaustive()
    }
}

impl BinStorage {
    pub const FILE_NAME: &str = "langram_models.bin";

    pub fn add(&mut self, name: String, mut model: Model) {
        let Some(lang) = ScriptLanguage::from_str(&name) else {
            return;
        };

        let model_wordgrams = ::core::mem::take(
            model
                .ngrams
                .get_safe_unchecked_mut(NgramSize::Word as usize),
        );

        if !model_wordgrams.is_empty() {
            let new_wordgram_min_probability =
                Model::compute_min_probability(model_wordgrams.len()) * 4.0;
            self.wordgram_min_probability = self
                .wordgram_min_probability
                .min(new_wordgram_min_probability);

            for (word, prob) in model_wordgrams {
                let entry = self.wordgrams.entry(word).or_default();
                entry.push((lang as u16, prob));
            }
        }

        for (ngram_size, model_ngrams) in model.ngrams.into_iter().enumerate() {
            let ngram_size = NgramSize::from(ngram_size);
            if ngram_size == NgramSize::Word {
                continue;
            }

            for (word, prob) in model_ngrams {
                let entry = self.ngrams.entry(word).or_default();
                entry.push((lang as u16, prob));
            }
        }

        *self
            .langs_ngram_min_probability
            .get_safe_unchecked_mut(lang as usize) = model.ngram_min_probability;
    }

    pub fn reorder(&mut self) {
        [&mut self.ngrams, &mut self.wordgrams]
            .into_iter()
            .flat_map(|v| v.iter_mut())
            .for_each(|(_, v)| {
                v.sort_by(|(l1, _), (l2, _)| {
                    ScriptLanguage::transmute_from_usize(*l1 as usize)
                        .cmp(&ScriptLanguage::transmute_from_usize(*l2 as usize))
                })
            });
    }

    #[inline]
    pub fn to_bytes(&self) -> Result<AlignedVec, rkyv::rancor::Error> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
    }
}
