use super::{builder::RealHasher, model::Model, DetectorBuilder, NgramSize};
use crate::{
    file_model::{load_model, parse_model, ChunksNgramsUnpacker, SpaceNgramsUnpacker},
    ngram_size::{NgramSizes, NgramSizesTrait},
};
use ::core::hash::BuildHasher;
use ::std::{
    collections::HashSet,
    fmt::{self, Debug},
};
use alphabet_detector::{ScriptLanguage, ScriptLanguageArr};
use debug_unsafe::slice::SliceGetter;
use parking_lot::{RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
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
    /// Preloads models for the provided languages.
    ///
    /// Not very efficient, because creates a temporary [`Detector`](struct.Detector.html).
    /// Better use [`Detector::preload_models`](struct.Detector.html#method.preload_models).
    pub fn preloaded<H: RealHasher>(languages: HashSet<ScriptLanguage, H>) -> Self {
        let models_storage = ModelsStorage::default();
        let detector = DetectorBuilder::new(&models_storage)
            .languages(languages)
            .build();
        detector.preload_models();
        models_storage
    }

    pub(super) fn load_model(
        &self,
        language: ScriptLanguage,
        ngram_size: NgramSize,
    ) -> RwLockReadGuard<'_, Model> {
        let lang_model = self.0.get_safe_unchecked(language as usize);
        let lang_model_guard = lang_model.upgradable_read();
        if lang_model_guard
            .ngrams
            .get_safe_unchecked(ngram_size as usize)
            .capacity()
            > 0
        {
            return RwLockUpgradableReadGuard::<'_, Model>::downgrade(lang_model_guard);
        }

        let mut lang_model_guard =
            RwLockUpgradableReadGuard::<'_, Model>::upgrade(lang_model_guard);
        let file_model = load_model(language, ngram_size);
        let ngram_model = parse_model::<_, ChunksNgramsUnpacker>(file_model, ngram_size);
        lang_model_guard.update_ngrams(ngram_model, ngram_size);

        RwLockWriteGuard::<'_, Model>::downgrade(lang_model_guard)
    }

    pub(super) fn load_wordgram_model(
        &self,
        language: ScriptLanguage,
    ) -> RwLockReadGuard<'_, Model> {
        let lang_model = self.0.get_safe_unchecked(language as usize);
        let lang_model_guard = lang_model.upgradable_read();
        if lang_model_guard.wordgrams.capacity() > 0 {
            return RwLockUpgradableReadGuard::<'_, Model>::downgrade(lang_model_guard);
        }

        let mut lang_model_guard =
            RwLockUpgradableReadGuard::<'_, Model>::upgrade(lang_model_guard);
        let file_model = load_model(language, NgramSize::Word);
        let wordgram_model = parse_model::<_, SpaceNgramsUnpacker>(file_model, NgramSize::Word);
        lang_model_guard.update_wordgrams(wordgram_model);

        RwLockWriteGuard::<'_, Model>::downgrade(lang_model_guard)
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
                _ = self.load_model(language, ngram_size);
            });

            if wordgrams_enabled {
                _ = self.load_wordgram_model(language);
            }
        };

        if PARALLEL {
            languages.par_iter().for_each(load);
        } else {
            languages.iter().for_each(load);
        };
    }

    #[inline]
    pub(super) fn load_unigram_models_for_languages(
        &self,
        languages: impl Iterator<Item = ScriptLanguage>,
    ) {
        languages.for_each(|language| {
            _ = self.load_model(language, NgramSize::Uni);
        });
    }

    /// Drops all models loaded
    pub fn unload(&self) {
        self.0.iter().for_each(|language_model| {
            *language_model.write() = Default::default();
        });
    }
}
