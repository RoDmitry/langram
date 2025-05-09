use crate::{
    ngram_size::{NgramSize, NgramSizes, NgramSizesTrait},
    ngrams::{prepare_ngrams, NgramString},
};
use ::core::cmp::Ordering;
use ::std::collections::HashSet;
use ahash::AHashSet;
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
pub(crate) use model::{ModelNgrams, NgramFromChars};
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
    #[inline]
    pub fn new(builder: DetectorBuilder<'m, H>) -> Self {
        Self {
            models_storage: builder.models_storage,
            languages: builder.languages.unwrap_safe_unchecked(),
            long_text_minlen: builder.long_text_minlen.unwrap_or(120),
            long_text_ngrams: builder.long_text_ngrams.unwrap_or_else(|| {
                NgramSizes::new_merged(
                    [
                        NgramSize::Tri,
                        NgramSize::Quadri,
                        NgramSize::Five,
                        NgramSize::Word,
                    ]
                    .into_iter(),
                )
            }),
            short_text_ngrams: builder.short_text_ngrams.unwrap_or_else(|| {
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
            }),
        }
    }

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

    /// Preload models for the languages selected in the config of this detector
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

    fn ngrams_sum_cnt<'a>(
        &'a self,
        language: ScriptLanguage,
        ngrams_iter: impl Iterator<Item = &'a str>,
        ngram_size: NgramSize,
    ) -> (f64, usize) {
        let language_model_lock = self
            .models_storage
            .0
            .get_safe_unchecked(language as usize)
            .read()
            .unwrap();

        let Some(language_model) = language_model_lock
            .ngrams
            .get(ngram_size as usize)
            .filter(|m| !m.is_empty())
        else {
            return (language_model_lock.ngram_min_probability, 1);
        };

        let mut sum = 0.0;
        let mut cnt = 0;
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
        &'a self,
        language: ScriptLanguage,
        ngrams_iter: impl Iterator<Item = &'a str>,
    ) -> (f64, usize) {
        let language_model_lock = self
            .models_storage
            .0
            .get_safe_unchecked(language as usize)
            .read()
            .unwrap();

        let language_model = &language_model_lock.wordgrams;
        if language_model.is_empty() {
            return (0.0, 0);
        };

        let mut sum = 0.0;
        let mut cnt = 0;
        for ngram in ngrams_iter {
            let probability = language_model
                .get(ngram)
                .copied()
                .inspect(|_| cnt += 1)
                .unwrap_or(language_model_lock.wordgram_min_probability);

            sum += probability;
        }

        (sum, cnt)
    }

    fn probabilities_languages_ngrams<'a>(
        &'a self,
        ngrams_iter: impl Iterator<Item = &'a str> + Clone,
        filtered_languages: &AHashSet<ScriptLanguage>,
        ngram_size: NgramSize,
        result: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        for &language in filtered_languages.iter() {
            self.models_storage.load_model(language, ngram_size);
            let ngrams_sum_cnt = self.ngrams_sum_cnt(language, ngrams_iter.clone(), ngram_size);
            result
                .get_safe_unchecked_mut(language as usize)
                .add(ngrams_sum_cnt);
        }
    }

    fn probabilities_languages_wordgrams<'a>(
        &'a self,
        ngrams_iter: impl Iterator<Item = &'a str> + Clone,
        filtered_languages: &AHashSet<ScriptLanguage>,
        result: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        for &language in filtered_languages.iter() {
            self.models_storage.load_wordgram_model(language);
            let ngrams_sum_cnt = self.wordgrams_sum_cnt(language, ngrams_iter.clone());
            result
                .get_safe_unchecked_mut(language as usize)
                .add(ngrams_sum_cnt);
        }
    }

    #[inline]
    fn compute<'a>(
        &'a self,
        words_iter: impl Iterator<Item = &'a [char]>,
        filtered_languages: &AHashSet<ScriptLanguage>,
        ngram_size: NgramSize,
        result: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        let ngrams = prepare_ngrams(words_iter, ngram_size);
        if ngrams.is_empty() {
            return;
        }

        self.probabilities_languages_ngrams(
            ngrams.iter().map(NgramString::as_str),
            filtered_languages,
            ngram_size,
            result,
        );
    }

    #[inline]
    fn sum_up_probabilities(
        probabilities: ScriptLanguageArr<(f64, usize)>,
        filtered_languages: AHashSet<ScriptLanguage>,
    ) -> Vec<(ScriptLanguage, f64)> {
        let mut res = Vec::with_capacity(filtered_languages.len());
        for language in filtered_languages.into_iter() {
            let (p, cnt) = *probabilities.get_safe_unchecked(language as usize);
            if cnt == 0 {
                continue;
            }

            res.push((language, p / cnt as f64));
        }

        res
    }

    /// Returns probabilities for the given text.
    /// Each value is a logarithmic probability between a negative infinity and 0.0.
    ///
    /// Result is sorted by probabilities in a descending order.
    /// If only single language is identified by `alphabet_detector`, the value 0.0 will be returned.
    pub fn probabilities(&self, text: &str) -> Vec<(ScriptLanguage, f64)> {
        if text.is_empty() {
            return Default::default();
        }

        let (words, langs) = fulltext_filter_with_margin::<Vec<char>, 95>(text.char_indices());
        let filtered_languages: AHashSet<_> = langs
            .filter(|(l, _)| self.languages.contains(l))
            .map(|(l, _)| l) // todo: maybe use count?
            .collect();

        if words.is_empty() || filtered_languages.is_empty() {
            return Default::default();
        }

        if filtered_languages.len() == 1 {
            let lang = filtered_languages.into_iter().next().unwrap();
            return vec![(lang, 0.0)];
        }

        let character_count: usize = words.iter().map(|wd| wd.buf.len()).sum();

        let mut ngram_sizes = if character_count >= self.long_text_minlen {
            self.long_text_ngrams.clone()
        } else {
            self.short_text_ngrams.clone()
        };

        /* if character_count < ngram_length_range.start {
            return filtered_languages
                .into_iter()
                .map(|l| (l, f64::NEG_INFINITY))
                .collect();
        } */

        // always preload unigrams
        if *ngram_sizes.first().unwrap() != NgramSize::Uni {
            self.models_storage
                .load_unigram_models_for_languages(&filtered_languages);
        }

        let wordgrams_enabled = *ngram_sizes.last().unwrap() == NgramSize::Word;
        if wordgrams_enabled {
            ngram_sizes.pop();
        }

        let mut probabilities = slang_arr_default::<(f64, usize)>();
        for ngram_size in ngram_sizes {
            self.compute(
                words.iter().map(|wd| wd.buf.as_ref()),
                &filtered_languages,
                ngram_size,
                &mut probabilities,
            );
        }

        if wordgrams_enabled {
            let wordgrams: Vec<String> = words.iter().map(|wd| wd.buf.iter().collect()).collect();
            self.probabilities_languages_wordgrams(
                wordgrams.iter().map(|s| s.as_str()),
                &filtered_languages,
                &mut probabilities,
            );
        }

        let mut probabilities_sums = Self::sum_up_probabilities(probabilities, filtered_languages);

        if probabilities_sums.is_empty() {
            return Default::default();
        }

        probabilities_sums.sort_unstable_by(order_by_probability_and_lang);
        /* println!(
            "res {:?}",
            &probabilities_sums[..probabilities_sums.len().min(5)]
        ); */

        probabilities_sums
    }

    /// Returns probabilities for the given text relative to other languages.
    /// Each value is a number between 0.0 and 1.0.
    ///
    /// If only single language is identified by `alphabet_detector`, the value 1.0 will be returned.
    pub fn probabilities_relative(&self, text: &str) -> Vec<(ScriptLanguage, f64)> {
        let mut probabilities = self.probabilities(text);
        transform_to_relative_probabilities(&mut probabilities);
        probabilities
    }

    /// Detects the top one language of the input text.
    /// If a single language cannot be returned, [`None`] is returned.
    pub fn detect_top_one(&self, text: &str, minimum_distance: f64) -> Option<ScriptLanguage> {
        debug_assert!(minimum_distance >= 0.0, "Minimum distance must be >= 0.0");
        let mut probabilities = self.probabilities(text).into_iter();

        let (most_likely_language, most_likely_language_probability) = probabilities.next()?;

        let Some((_, second_most_likely_language_probability)) = probabilities.next() else {
            return Some(most_likely_language);
        };

        let language_probability_diff =
            (most_likely_language_probability - second_most_likely_language_probability).abs();

        if language_probability_diff.is_nan()
            || language_probability_diff < f64::EPSILON
            || language_probability_diff < minimum_distance
        {
            return None;
        }

        Some(most_likely_language)
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
fn transform_to_relative_probabilities(probabilities: &mut Vec<(ScriptLanguage, f64)>) {
    if probabilities.is_empty() {
        return;
    }

    debug_assert!(!probabilities.iter().any(|(_, p)| p.is_nan()));

    let first_probability = probabilities.first().unwrap().1;
    if first_probability.is_zero() {
        let zeroes = probabilities
            .iter()
            .position(|(_, p)| !p.is_zero())
            .unwrap_or(probabilities.len());
        probabilities.truncate(zeroes);
        let len = zeroes as f64;
        probabilities.iter_mut().for_each(|(_, p)| *p = 1.0 / len);

        return;
    }

    if first_probability == f64::NEG_INFINITY {
        let len = probabilities.len() as f64;
        probabilities.iter_mut().for_each(|(_, p)| *p = 1.0 / len);

        return;
    }

    probabilities.iter_mut().for_each(|(_, p)| *p = p.exp());
    let denominator: f64 = probabilities.iter().map(|(_, p)| *p).sum();

    if denominator.is_zero() {
        if let Some((_, p)) = probabilities.first_mut() {
            *p = 1.0
        }
        probabilities.truncate(1);
    } else {
        probabilities
            .iter_mut()
            .for_each(|(_, p)| *p /= denominator);
    }
}
