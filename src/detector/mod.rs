use crate::{
    ngram_size::{NgramSize, NgramSizes, NgramSizesTrait},
    ngrams::{prepare_ngrams, NgramString},
};
use ::core::cmp::Ordering;
use ::std::collections::HashSet;
use alphabet_detector::{
    fulltext_filter_with_margin, slang_arr_default, ScriptLanguage, ScriptLanguageArr,
};
use debug_unsafe::{option::OptionUnwrapper, slice::SliceGetter};
use fraction::Zero;

mod builder;
mod model;
mod storage;
#[cfg(test)]
mod tests;

pub use builder::DetectorBuilder;
use builder::RealHasher;
use model::Model;
pub(crate) use model::{ModelNgrams, NgramFromChars};
use parking_lot::RwLockReadGuard;
pub use storage::ModelsStorage;

trait ProbabilitiesAdder: Sized {
    fn add(&mut self, add: (f64, usize));
}

impl ProbabilitiesAdder for (f64, usize) {
    #[inline(always)]
    fn add(&mut self, add: (f64, usize)) {
        self.0 += add.0;
        self.1 += add.1;
    }
}

#[derive(Clone, Debug)]
pub struct Detector<'m, H: RealHasher> {
    models_storage: &'m ModelsStorage,
    pub languages: HashSet<ScriptLanguage, H>,
    pub long_text_minlen: usize,
    long_text_ngrams: NgramSizes,
    short_text_ngrams: NgramSizes,
}

impl<'m, H: RealHasher> Detector<'m, H> {
    /// Will have all ngrams enabled if none selected
    #[inline]
    fn new(builder: DetectorBuilder<'m, H>) -> Self {
        let long_text_ngrams = if !builder.long_text_ngrams.is_empty() {
            builder.long_text_ngrams
        } else {
            NgramSizes::new_merged(
                [
                    NgramSize::Tri,
                    NgramSize::Quadri,
                    NgramSize::Five,
                    NgramSize::Word,
                ]
                .into_iter(),
            )
        };

        let short_text_ngrams = if !builder.short_text_ngrams.is_empty() {
            builder.short_text_ngrams
        } else {
            NgramSizes::new_merged(
                [
                    NgramSize::Uni,
                    NgramSize::Bi,
                    NgramSize::Tri,
                    NgramSize::Quadri,
                    NgramSize::Five,
                    NgramSize::Word,
                ]
                .into_iter(),
            )
        };

        Self {
            models_storage: builder.models_storage,
            languages: builder.languages,
            long_text_minlen: builder.long_text_minlen,
            long_text_ngrams,
            short_text_ngrams,
        }
    }

    /// Clone detector with new languages selected
    #[inline]
    pub fn clone_with_languages<H2: RealHasher>(
        &self,
        languages: HashSet<ScriptLanguage, H2>,
    ) -> Detector<'m, H2> {
        Detector {
            models_storage: self.models_storage,
            languages,
            long_text_minlen: self.long_text_minlen,
            long_text_ngrams: self.long_text_ngrams.clone(),
            short_text_ngrams: self.short_text_ngrams.clone(),
        }
    }

    /// Preloads models for the languages selected in this detector
    pub fn preload_models(&self) {
        let mut ngram_sizes = self.short_text_ngrams.clone();
        ngram_sizes.merge(self.long_text_ngrams.iter().copied());

        #[cfg(not(target_family = "wasm"))]
        const PARALLEL: bool = true;
        #[cfg(target_family = "wasm")]
        const PARALLEL: bool = false;

        self.models_storage
            .load_models_for_languages::<PARALLEL, _>(ngram_sizes, &self.languages);
    }

    /// Drops all models loaded
    #[inline]
    pub fn unload_models(&self) {
        self.models_storage.unload();
    }

    fn ngrams_sum_cnt<'n>(
        language_model_lock: RwLockReadGuard<'_, Model>,
        ngrams_iter: impl Iterator<Item = &'n str>,
        ngram_size: NgramSize,
    ) -> (f64, usize) {
        let mut cnt = 0;

        let Some(language_model) = language_model_lock
            .ngrams
            .get(ngram_size as usize)
            .filter(|m| !m.is_empty())
        else {
            return (language_model_lock.ngram_min_probability, cnt);
        };

        let mut sum = 0.0;
        for ngram in ngrams_iter {
            let probability = language_model
                .get(ngram)
                .copied()
                .inspect(|_| cnt += 1)
                .unwrap_or(language_model_lock.ngram_min_probability);

            sum += probability;
        }

        (sum, cnt)
    }

    fn wordgrams_sum_cnt<'a>(
        language_model_lock: RwLockReadGuard<'_, Model>,
        ngrams_iter: impl Iterator<Item = &'a str>,
    ) -> (f64, usize) {
        let mut sum = 0.0;
        let mut cnt = 0;

        let language_model = &language_model_lock.wordgrams;
        if language_model.is_empty() {
            return (sum, cnt);
        };

        for ngram in ngrams_iter {
            let probability = language_model
                .get(ngram)
                .copied()
                .inspect(|_| cnt += 1)
                .unwrap_or_else(|| *language_model_lock.wordgram_min_probability.read());

            sum += probability;
        }

        (sum, cnt)
    }

    fn probabilities_languages_ngrams<'a>(
        models_storage: &'m ModelsStorage,
        ngrams_iter: impl Iterator<Item = &'a str> + Clone,
        languages: impl Iterator<Item = ScriptLanguage>,
        ngram_size: NgramSize,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        for language in languages {
            let language_model_lock = models_storage.load_model(language, ngram_size);
            let ngrams_sum_cnt =
                Self::ngrams_sum_cnt(language_model_lock, ngrams_iter.clone(), ngram_size);
            output
                .get_safe_unchecked_mut(language as usize)
                .add(ngrams_sum_cnt);
        }
    }

    fn probabilities_languages_wordgrams<'a>(
        models_storage: &'m ModelsStorage,
        ngrams_iter: impl Iterator<Item = &'a str> + Clone,
        languages: impl Iterator<Item = ScriptLanguage>,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        for language in languages {
            let language_model_lock = models_storage.load_wordgram_model(language);
            let ngrams_sum_cnt = Self::wordgrams_sum_cnt(language_model_lock, ngrams_iter.clone());
            output
                .get_safe_unchecked_mut(language as usize)
                .add(ngrams_sum_cnt);
        }
    }

    #[inline]
    fn probabilities_ngrams<'a>(
        models_storage: &'m ModelsStorage,
        words_iter: impl Iterator<Item = &'a [char]>,
        languages: impl Iterator<Item = ScriptLanguage>,
        ngram_size: NgramSize,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        let ngrams = prepare_ngrams(words_iter, ngram_size);
        if ngrams.is_empty() {
            return;
        }

        Self::probabilities_languages_ngrams(
            models_storage,
            ngrams.iter().map(NgramString::as_str),
            languages,
            ngram_size,
            output,
        );

        /* let mut dbg: Vec<_> = output
            .iter()
            .enumerate()
            .filter(|(_, d)| d.1 > 0)
            .map(|(l, d)| (ScriptLanguage::transmute_from_usize(l), d))
            .collect();
        dbg.sort_by(|(_, d1), (_, d2)| d2.1.cmp(&d1.1).then(d2.0.total_cmp(&d1.0)));
        println!("OUTPUT {:?}", dbg); */
    }

    /// Computes mean average for each language
    #[inline]
    fn probabilities_mean(
        probabilities: ScriptLanguageArr<(f64, usize)>,
        filtered_languages: Vec<ScriptLanguage>,
    ) -> Vec<(ScriptLanguage, f64)> {
        let top_cnt = filtered_languages
            .iter()
            .map(|l| probabilities.get_safe_unchecked(*l as usize).1)
            .max()
            .unwrap_or_default()
            .min(61);

        let mut res = Vec::with_capacity(filtered_languages.len());
        for language in filtered_languages.into_iter() {
            let (p, cnt) = *probabilities.get_safe_unchecked(language as usize);
            res.push((
                language,
                if cnt == 0 || cnt < top_cnt {
                    f64::NEG_INFINITY
                } else {
                    p / cnt as f64
                },
            ));
        }

        res
    }

    /// Returns probabilities for the provided text.
    /// Each value is a logarithmic probability between a negative infinity and 0.0.
    ///
    /// Result is sorted by probabilities in a descending order.
    ///
    /// If only a single language is identified by `alphabet_detector`,
    /// the value 0.0 will be returned.
    pub fn probabilities(&self, text: &str) -> Vec<(ScriptLanguage, f64)> {
        if text.is_empty() {
            return Default::default();
        }

        let (words, langs, _) = fulltext_filter_with_margin::<Vec<char>, 95>(text.char_indices());
        let filtered_languages: Vec<_> = langs
            .filter(|(l, _)| self.languages.contains(l))
            .map(|(l, _)| l)
            .collect();

        if words.is_empty() || filtered_languages.is_empty() {
            return Default::default();
        }

        if filtered_languages.len() == 1 {
            let lang = filtered_languages
                .into_iter()
                .next()
                .unwrap_safe_unchecked();
            return vec![(lang, 0.0)];
        }

        let characters_count: usize = words.iter().map(|wd| wd.buf.len()).sum();

        let ngram_sizes = if characters_count < self.long_text_minlen {
            &self.short_text_ngrams
        } else {
            &self.long_text_ngrams
        };
        debug_assert!(!ngram_sizes.is_empty());
        let mut ngram_sizes_len = ngram_sizes.len();

        /* if characters_count < ngram_length_range.start {
            return filtered_languages
                .into_iter()
                .map(|l| (l, f64::NEG_INFINITY))
                .collect();
        } */

        // always preload unigrams
        if *ngram_sizes.first().unwrap_safe_unchecked() != NgramSize::Uni {
            self.models_storage
                .load_unigram_models_for_languages(filtered_languages.iter().copied());
        }

        let wordgrams_enabled = *ngram_sizes.last().unwrap_safe_unchecked() == NgramSize::Word;
        if wordgrams_enabled {
            ngram_sizes_len -= 1;
        }

        let mut probabilities = slang_arr_default::<(f64, usize)>();
        for &ngram_size in ngram_sizes.get_safe_unchecked(..ngram_sizes_len) {
            Self::probabilities_ngrams(
                self.models_storage,
                words.iter().map(|wd| wd.buf.as_ref()),
                filtered_languages.iter().copied(),
                ngram_size,
                &mut probabilities,
            );
        }

        if wordgrams_enabled {
            let wordgrams: Vec<String> = words.iter().map(|wd| wd.buf.iter().collect()).collect();
            Self::probabilities_languages_wordgrams(
                self.models_storage,
                wordgrams.iter().map(|s| s.as_str()),
                filtered_languages.iter().copied(),
                &mut probabilities,
            );
        }

        let mut probabilities_mean = Self::probabilities_mean(probabilities, filtered_languages);

        probabilities_mean.sort_unstable_by(order_by_probability_and_lang);
        /* println!(
            "res {:?}",
            &probabilities_mean[..probabilities_mean.len().min(5)]
        ); */

        probabilities_mean
    }

    /// Returns probabilities for the provided text relative to other languages.
    /// Each value is a number between 0.0 and 1.0.
    ///
    /// If only a single language is identified by `alphabet_detector`,
    /// the value 1.0 will be returned.
    pub fn probabilities_relative(&self, text: &str) -> Vec<(ScriptLanguage, f64)> {
        let mut probabilities = self.probabilities(text);
        transform_to_relative_probabilities(&mut probabilities);
        probabilities
    }

    /// Detects a top one language of the provided text.
    ///
    /// `minimum_distance` is a distance between a first and a second logarithmic probabilities,
    /// which can help filter languages with close probabilities.
    ///
    /// If a single language cannot be returned, [`None`] is returned.
    pub fn detect_top_one(&self, text: &str, minimum_distance: f64) -> Option<ScriptLanguage> {
        debug_assert!(minimum_distance >= 0.0, "Minimum distance must be >= 0.0");

        let mut probabilities = self.probabilities(text).into_iter();

        let (first_language, first_probability) = probabilities.next()?;
        let Some((_, second_probability)) = probabilities.next() else {
            return Some(first_language);
        };

        let probabilities_diff = first_probability - second_probability;
        if probabilities_diff.is_nan()
            || probabilities_diff < f64::EPSILON
            || probabilities_diff < minimum_distance
        {
            return None;
        }

        Some(first_language)
    }
}

#[inline]
fn order_by_probability_and_lang(
    first: &(ScriptLanguage, f64),
    second: &(ScriptLanguage, f64),
) -> Ordering {
    second
        .1
        .total_cmp(&first.1)
        .then_with(|| first.0.cmp(&second.0))
}

/// `probabilities` must be ordered
#[inline]
fn transform_to_relative_probabilities(probabilities: &mut Vec<(ScriptLanguage, f64)>) {
    if probabilities.is_empty() {
        return;
    }

    debug_assert!(!probabilities.iter().any(|(_, p)| p.is_nan()));

    let first_probability = probabilities.first().unwrap_safe_unchecked().1;
    if first_probability.is_zero() {
        let zeroes = probabilities
            .iter()
            .position(|(_, p)| !p.is_zero())
            .unwrap_or(probabilities.len());
        probabilities.truncate(zeroes);
    }

    if first_probability.is_zero() || first_probability == f64::NEG_INFINITY {
        let len = probabilities.len() as f64;
        probabilities.iter_mut().for_each(|(_, p)| *p = 1.0 / len);

        return;
    }

    let mut denominator: f64 = 0.0;
    probabilities.iter_mut().for_each(|(_, p)| {
        *p = p.exp();
        denominator += *p;
    });

    if denominator.is_zero() {
        // ::core::hint::cold_path();
        if let Some((_, p)) = probabilities.first_mut() {
            *p = 1.0
        }
        probabilities.truncate(1);

        return;
    }

    probabilities
        .iter_mut()
        .for_each(|(_, p)| *p /= denominator);
}
