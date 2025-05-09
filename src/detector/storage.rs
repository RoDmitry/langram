use super::{builder::RealHasher, model::Model, DetectorBuilder, NgramSize};
use crate::{
    file_model::{load_model, parse_model, ChunksNgramsUnpacker, SpaceNgramsUnpacker},
    ngram_size::{NgramSizes, NgramSizesTrait},
};
use ::core::hash::BuildHasher;
use ::std::{
    collections::HashSet,
    fmt::{self, Debug},
    sync::RwLock,
};
use alphabet_detector::{ScriptLanguage, ScriptLanguageArr};
use debug_unsafe::slice::SliceGetter;
#[cfg(not(target_family = "wasm"))]
use rayon::prelude::*;

type LanguagesModels = ScriptLanguageArr<RwLock<Model>>;

/// With all models preloaded uses around 4.1GB of RAM.
pub struct ModelsStorage(pub(super) LanguagesModels);

impl Default for ModelsStorage {
    #[inline]
    fn default() -> Self {
        Self(::core::array::from_fn(|_| Default::default()))
    }
}

impl Debug for ModelsStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelsStorage").finish_non_exhaustive()
    }
}

impl ModelsStorage {
    pub fn preloaded<H: RealHasher>(languages: HashSet<ScriptLanguage, H>) -> Self {
        let models_storage = ModelsStorage::default();
        let detector = DetectorBuilder::new(&models_storage)
            .languages(languages)
            .build();
        detector.preload_models();
        models_storage
    }

    pub(super) fn load_model(&self, language: ScriptLanguage, ngram_size: NgramSize) {
        let lang_model = self.0.get_safe_unchecked(language as usize);
        let lang_model_guard = lang_model.read().unwrap();
        if lang_model_guard
            .ngrams
            .get_safe_unchecked(ngram_size as usize)
            .capacity()
            > 0
        {
            return;
        }

        drop(lang_model_guard);
        let mut lang_model_guard = lang_model.write().unwrap();
        // second check here, because there can be multiple threads waiting for the write lock
        if lang_model_guard
            .ngrams
            .get_safe_unchecked(ngram_size as usize)
            .capacity()
            > 0
        {
            return;
        }
        let file_model = load_model(language, ngram_size.into_file_name());
        let ngram_model = parse_model::<_, ChunksNgramsUnpacker>(file_model, ngram_size);
        lang_model_guard.update_ngrams(ngram_model, ngram_size);
    }

    pub(super) fn load_wordgram_model(&self, language: ScriptLanguage) {
        let lang_model = self.0.get_safe_unchecked(language as usize);
        let lang_model_guard = lang_model.read().unwrap();
        if lang_model_guard.wordgrams.capacity() > 0 {
            return;
        }

        drop(lang_model_guard);
        let mut lang_model_guard = lang_model.write().unwrap();
        // second check here, because there can be multiple threads waiting for the write lock
        if lang_model_guard.wordgrams.capacity() > 0 {
            return;
        }
        let file_model = load_model(language, NgramSize::Word.into_file_name());
        let wordgram_model = parse_model::<_, SpaceNgramsUnpacker>(file_model, NgramSize::Word);
        lang_model_guard.update_wordgrams(wordgram_model);
    }

    pub(super) fn load_models_for_languages<const PARALLEL: bool, H: BuildHasher>(
        &self,
        mut ngram_sizes: NgramSizes,
        languages: &HashSet<ScriptLanguage, H>,
    ) {
        // always load unigrams
        if *ngram_sizes.first().unwrap() != NgramSize::Uni {
            ngram_sizes.merge([NgramSize::Uni].into_iter());
        }

        let wordgrams_enabled = *ngram_sizes.last().unwrap() == NgramSize::Word;
        if wordgrams_enabled {
            ngram_sizes.pop();
        }

        let load = move |&language| {
            ngram_sizes.iter().for_each(|&ngram_size| {
                self.load_model(language, ngram_size);
            });

            if wordgrams_enabled {
                self.load_wordgram_model(language);
            }
        };

        if PARALLEL {
            languages.par_iter().for_each(load);
        } else {
            languages.iter().for_each(load);
        };
    }

    #[inline]
    pub(super) fn load_unigram_models_for_languages<H: BuildHasher>(
        &self,
        languages: &HashSet<ScriptLanguage, H>,
    ) {
        languages.iter().for_each(|&language| {
            self.load_model(language, NgramSize::Uni);
        });
    }

    /// Drops all models loaded
    pub fn unload(&self) {
        self.0.iter().for_each(|language_model| {
            *language_model.write().unwrap() = Default::default();
        });
    }
}
