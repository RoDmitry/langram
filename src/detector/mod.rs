use crate::{
    ngram_size::{NgramSize, NgramSizes, NgramSizesTrait},
    ngrams::{prepare_ngrams, NgramString},
};
use ::core::cmp::Ordering;
use ::std::collections::HashSet;
use ahash::AHashSet;
use alphabet_detector::{
    filter_with_margin, fulltext_filter_with_margin, slang_arr_default, ScriptLanguage,
    ScriptLanguageArr,
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

        let mut sum = 0.0;
        let mut cnt = 0;

        let Some(language_model) = language_model_lock
            .ngrams
            .get(ngram_size as usize)
            .filter(|m| !m.is_empty())
        else {
            return (sum, cnt);
        };

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

    fn wordgrams_sum_cnt(&self, language: ScriptLanguage, word: &str) -> (f64, usize) {
        let language_model_lock = self
            .models_storage
            .0
            .get_safe_unchecked(language as usize)
            .read()
            .unwrap();

        let mut cnt = 0;

        let language_model = &language_model_lock.wordgrams;
        if language_model.is_empty() {
            return (0.0, cnt);
        };

        let probability = language_model
            .get(word)
            .copied()
            .inspect(|_| cnt += 1)
            .unwrap_or(language_model_lock.wordgram_min_probability);

        (probability, cnt)
    }

    fn probabilities_languages_ngrams<'a>(
        &'a self,
        ngrams_iter: impl Iterator<Item = &'a str> + Clone,
        languages: impl Iterator<Item = ScriptLanguage>,
        ngram_size: NgramSize,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        for language in languages {
            self.models_storage.load_model(language, ngram_size);
            let ngrams_sum_cnt = self.ngrams_sum_cnt(language, ngrams_iter.clone(), ngram_size);
            output
                .get_safe_unchecked_mut(language as usize)
                .add(ngrams_sum_cnt);
        }
    }

    fn probabilities_languages_wordgrams(
        &self,
        word: &str,
        languages: impl Iterator<Item = ScriptLanguage>,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        for language in languages {
            self.models_storage.load_wordgram_model(language);
            let ngrams_sum_cnt = self.wordgrams_sum_cnt(language, word);
            output
                .get_safe_unchecked_mut(language as usize)
                .add(ngrams_sum_cnt);
        }
    }

    #[inline]
    fn compute<'a>(
        &'a self,
        word: &'a [char],
        languages: impl Iterator<Item = ScriptLanguage>,
        ngram_size: NgramSize,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        let ngrams = prepare_ngrams(word, ngram_size);
        if ngrams.is_empty() {
            return;
        }

        self.probabilities_languages_ngrams(
            ngrams.iter().map(NgramString::as_str),
            languages,
            ngram_size,
            output,
        );
    }

    /// Computes mean average for each language
    #[inline]
    fn probabilities_mean(
        probabilities: ScriptLanguageArr<(f64, usize)>,
        languages: impl Iterator<Item = ScriptLanguage> + Clone,
        output: &mut ScriptLanguageArr<Option<f64>>,
    ) {
        let top = languages
            .clone()
            .map(|l| probabilities.get_safe_unchecked(l as usize).1)
            .max()
            .unwrap_or_default();

        if top == 0 {
            languages
                .for_each(|l| *output.get_safe_unchecked_mut(l as usize) = Some(f64::NEG_INFINITY));
        } else {
            for language in languages {
                let (p, cnt) = *probabilities.get_safe_unchecked(language as usize);
                let res = if cnt < top {
                    f64::NEG_INFINITY
                } else {
                    p / cnt as f64
                };

                match output.get_safe_unchecked_mut(language as usize) {
                    Some(v) => *v += res,
                    v => *v = Some(res),
                }
            }
        }
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

        let (words, langs) = fulltext_filter_with_margin::<Vec<char>, 95>(text.char_indices());
        let filtered_languages: AHashSet<_> = langs
            .filter(|(l, _)| self.languages.contains(l))
            .map(|(l, _)| l) // todo: maybe use count?
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
                .load_unigram_models_for_languages(&filtered_languages);
        }

        let wordgrams_enabled = *ngram_sizes.last().unwrap_safe_unchecked() == NgramSize::Word;
        if wordgrams_enabled {
            ngram_sizes_len -= 1;
        }

        let mut probabilities_mean = slang_arr_default::<Option<f64>>();
        for wd in words.into_iter() {
            let word_languages: Vec<_> = filter_with_margin::<95>(wd.langs_cnt)
                .map(|(l, _)| l)
                .filter(|l| filtered_languages.contains(l))
                .collect();

            let mut probabilities = slang_arr_default::<(f64, usize)>();
            let word = wd.buf.as_ref();
            for &ngram_size in ngram_sizes.get_safe_unchecked(..ngram_sizes_len) {
                self.compute(
                    word,
                    word_languages.iter().copied(),
                    ngram_size,
                    &mut probabilities,
                );
            }

            if wordgrams_enabled {
                let word: String = wd.buf.iter().collect();
                self.probabilities_languages_wordgrams(
                    &word,
                    word_languages.iter().copied(),
                    &mut probabilities,
                );
            }

            Self::probabilities_mean(
                probabilities,
                word_languages.iter().copied(),
                &mut probabilities_mean,
            );
        }

        let mut probabilities_mean: Vec<_> = probabilities_mean
            .into_iter()
            .enumerate()
            .filter_map(|(l, p)| p.map(|p2| (ScriptLanguage::transmute_from_usize(l), p2)))
            .collect();
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
