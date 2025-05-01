use super::{model::Model, Detector, DetectorConfig, NgramsSize};
use crate::file_model::{load_model, parse_model};
use ::core::hash::BuildHasher;
use ::std::{collections::HashSet, ops::RangeInclusive, sync::RwLock};
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
    pub fn preloaded<H: BuildHasher + Default + Sync>(
        languages: HashSet<ScriptLanguage, H>,
    ) -> Self {
        let models_storage = ModelsStorage::default();
        let config = DetectorConfig::with_languages(languages);
        let detector = Detector::new(config, &models_storage);
        detector.preload_models();
        models_storage
    }

    fn load_model(&self, language: ScriptLanguage, ngram_size: NgramsSize) {
        let ngram_models = self.0.get_safe_unchecked(language as usize);
        let ngram_models_guard = ngram_models.read().unwrap();
        if ngram_models_guard
            .ngrams
            .get_safe_unchecked(ngram_size as usize)
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
            .get_safe_unchecked(ngram_size as usize)
            .capacity()
            > 0
        {
            return;
        }
        let file_model = load_model(language, ngram_size.into_file_name());
        let ngram_model = parse_model(file_model, ngram_size);
        ngram_models_guard.update_ngrams(ngram_model, ngram_size);
    }

    pub(super) fn load_models_from_languages<const PARALLEL: bool, HL: BuildHasher>(
        &self,
        ngram_length_range: RangeInclusive<usize>,
        languages: &HashSet<ScriptLanguage, HL>,
    ) {
        let load = move |&language| {
            // always load unigrams
            if *ngram_length_range.start() > 1 {
                self.load_model(language, NgramsSize::Uni);
            }
            ngram_length_range.clone().for_each(|ngram_length| {
                self.load_model(language, NgramsSize::from(ngram_length - 1))
            });
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
