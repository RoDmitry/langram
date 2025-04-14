use super::{model::Model, Detector, DetectorConfig};
use crate::{
    file_model::{load_model, parse_model},
    NGRAM_MAX_SIZE,
};
use ::core::hash::BuildHasher;
use ::std::{collections::HashSet, ops::RangeInclusive, sync::RwLock};
use ahash::AHashMap;
use alphabet_detector::{ScriptLanguage, ScriptLanguageArr};
use debug_unsafe::slice::SliceGetter;
#[cfg(not(target_family = "wasm"))]
use rayon::prelude::*;

type LanguagesModels = ScriptLanguageArr<RwLock<Model>>;

pub struct ModelsStorage(pub(super) LanguagesModels);

impl Default for ModelsStorage {
    #[inline]
    fn default() -> Self {
        Self(::core::array::from_fn(|_| Default::default()))
    }
}

impl ModelsStorage {
    pub fn preloaded<S: BuildHasher + Default + Sync>(
        languages: HashSet<ScriptLanguage, S>,
    ) -> Self {
        let models_storage = ModelsStorage::default();
        let config = DetectorConfig::with_languages(languages);
        let detector = Detector::new(config, &models_storage);
        detector.preload_models();
        models_storage
    }

    fn load_model(&self, language: ScriptLanguage, ngram_length: usize) {
        debug_assert!(
            (1..=NGRAM_MAX_SIZE).contains(&ngram_length),
            "ngram length {ngram_length} is not in range 1..={NGRAM_MAX_SIZE}"
        );

        let ngram_models = self.0.get_safe_unchecked(language as usize);
        let index = ngram_length - 1;
        let ngram_models_guard = ngram_models.read().unwrap();
        if ngram_models_guard
            .ngrams
            .get_safe_unchecked(index)
            .capacity()
            > 0
        {
            return;
        }

        drop(ngram_models_guard);
        let mut ngram_models_guard = ngram_models.write().unwrap();
        // second check here, because there can be multiple threads waiting for the write lock
        if ngram_models_guard
            .ngrams
            .get_safe_unchecked(index)
            .capacity()
            > 0
        {
            return;
        }
        let file_model = load_model(language, ngram_length);
        let ngram_model = match file_model {
            Ok(m) => parse_model(m, ngram_length),
            _ => AHashMap::with_capacity(1),
        };
        ngram_models_guard.update_ngram(ngram_model, index);
    }

    pub(super) fn load_models_from_languages<const PARALLEL: bool, HL: BuildHasher>(
        &self,
        ngram_length_range: RangeInclusive<usize>,
        languages: &HashSet<ScriptLanguage, HL>,
    ) {
        let load = move |&language| {
            // always load unigrams
            if *ngram_length_range.start() > 1 {
                self.load_model(language, 1);
            }
            ngram_length_range
                .clone()
                .for_each(|ngram_length| self.load_model(language, ngram_length));
        };

        if PARALLEL {
            languages.par_iter().for_each(load);
        } else {
            languages.iter().for_each(load);
        };
    }

    /// Drops all models loaded
    pub fn unload(&self) {
        self.0.iter().for_each(|language_model| {
            *language_model.write().unwrap() = Default::default();
        });
    }
}
