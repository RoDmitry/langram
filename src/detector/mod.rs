use crate::{
    detector::storage::NgramModel,
    ngram_size::{NgramSize, NgramSizes, NgramSizesTrait},
    ngrams::ngram_iterator,
};
use ::core::cmp::Ordering;
use ::std::borrow::Borrow;
use alphabet_detector::{
    fulltext_filter_with_margin, slang_arr_default, ScriptLanguage, ScriptLanguageArr, Word,
};
use debug_unsafe::{option::OptionUnwrapper, slice::SliceGetter};

mod builder;
#[cfg(all(debug_assertions, test))]
mod mock_tests;
mod storage;

pub use builder::DetectorBuilder;
use rkyv::tuple::ArchivedTuple2;
use rustc_hash::FxHashSet;
pub use storage::{ModelsStorage, ModelsStorageError};

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
pub struct Detector<'m> {
    models_storage: &'m ModelsStorage<'m>,
    pub languages: FxHashSet<ScriptLanguage>,
    pub long_text_minlen: usize,
    long_text_ngram_sizes: NgramSizes,
    short_text_ngram_sizes: NgramSizes,
}

impl<'m> Detector<'m> {
    /// Will have all ngrams enabled if none selected
    #[inline]
    fn new<L>(builder: DetectorBuilder<'m, L>) -> Self
    where
        L: IntoIterator<Item = ScriptLanguage>,
    {
        let long_text_ngram_sizes = if !builder.long_text_ngram_sizes.is_empty() {
            builder.long_text_ngram_sizes
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

        let short_text_ngram_sizes = if !builder.short_text_ngram_sizes.is_empty() {
            builder.short_text_ngram_sizes
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
            languages: builder.languages.into_iter().collect(),
            long_text_minlen: builder.long_text_minlen,
            long_text_ngram_sizes,
            short_text_ngram_sizes,
        }
    }

    /// Clone detector with new languages selected
    #[inline]
    pub fn clone_with_languages(&self, languages: FxHashSet<ScriptLanguage>) -> Detector<'m> {
        Detector {
            models_storage: self.models_storage,
            languages,
            long_text_minlen: self.long_text_minlen,
            long_text_ngram_sizes: self.long_text_ngram_sizes.clone(),
            short_text_ngram_sizes: self.short_text_ngram_sizes.clone(),
        }
    }

    #[inline]
    fn ngrams_sum_cnt(
        ngram_model: &'m NgramModel,
        ngrams_iter: impl Iterator<Item = impl Borrow<str>>,
        languages: &FxHashSet<ScriptLanguage>,
        output: &mut ScriptLanguageArr<(f64, usize)>,
        min_prob_getter: impl Fn(ScriptLanguage) -> f64,
    ) {
        for ngram in ngrams_iter {
            let Some(langs_probs) = ngram_model.get(ngram.borrow()).filter(|v| !v.is_empty())
            else {
                continue;
            };

            let mut languages_tmp = languages.clone();
            for ArchivedTuple2(language, prob) in langs_probs.iter() {
                let language = ScriptLanguage::transmute_from_usize(language.to_native() as usize);
                if !languages_tmp.remove(&language) {
                    continue;
                }
                let prob = prob.to_native();

                output
                    .get_safe_unchecked_mut(language as usize)
                    .add((prob, 1));
            }

            languages_tmp.into_iter().for_each(|language| {
                output.get_safe_unchecked_mut(language as usize).0 += min_prob_getter(language);
            });
        }
    }

    fn probabilities_languages_ngrams(
        models_storage: &'m ModelsStorage,
        ngram_size: NgramSize,
        ngrams_iter: impl Iterator<Item = impl Borrow<str>>,
        languages: &FxHashSet<ScriptLanguage>,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        Self::ngrams_sum_cnt(
            models_storage
                .ngrams
                .get_safe_unchecked(ngram_size as usize),
            ngrams_iter,
            languages,
            output,
            #[inline]
            |language| {
                models_storage
                    .langs_ngram_min_probability
                    .get_safe_unchecked(language as usize)
                    .to_native()
            },
        );
    }

    fn probabilities_languages_wordgrams(
        models_storage: &'m ModelsStorage,
        ngrams_iter: impl Iterator<Item = impl Borrow<str>>,
        languages: &FxHashSet<ScriptLanguage>,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        Self::ngrams_sum_cnt(
            models_storage.wordgrams,
            ngrams_iter,
            languages,
            output,
            #[inline]
            |_| models_storage.wordgram_min_probability,
        );
    }

    /// faster with this function, maybe because of the lifetime 'a
    #[inline(always)]
    fn probabilities_ngrams<'a>(
        models_storage: &'m ModelsStorage,
        words_iter: impl Iterator<Item = &'a [char]>,
        languages: &FxHashSet<ScriptLanguage>,
        ngram_size: NgramSize,
        output: &mut ScriptLanguageArr<(f64, usize)>,
    ) {
        let ngrams_iter = ngram_iterator::<'a>(words_iter, ngram_size);

        Self::probabilities_languages_ngrams(
            models_storage,
            ngram_size,
            ngrams_iter,
            languages,
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
        filtered_languages: FxHashSet<ScriptLanguage>,
    ) -> Vec<(ScriptLanguage, f64)> {
        let mut res = Vec::with_capacity(filtered_languages.len());
        for language in filtered_languages.into_iter() {
            let (p, cnt) = *probabilities.get_safe_unchecked(language as usize);
            res.push((
                language,
                if cnt == 0 {
                    f64::NEG_INFINITY
                } else {
                    p / cnt as f64
                },
            ));
        }

        res
    }

    /// Returns probabilities for the provided text.
    /// Each value of `probabilities` in `ProbabilitiesExtra` is a logarithmic probability
    /// between a negative infinity and 0.0. Also contains words.
    ///
    /// Result is sorted by probabilities in a descending order.
    ///
    /// If only a single language is identified by `alphabet_detector`,
    /// the value 0.0 will be returned.
    fn probabilities_extra(&self, text: &str) -> ProbabilitiesExtra {
        if text.is_empty() {
            return Default::default();
        }

        let (words, langs, _) = fulltext_filter_with_margin::<Vec<char>, 95>(text.char_indices());
        let filtered_languages: FxHashSet<_> = langs
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

            return ProbabilitiesExtra {
                probabilities: vec![(lang, 0.0)],
                words,
            };
        }

        let characters_count: usize = words.iter().map(|wd| wd.buf.len()).sum();

        let mut ngram_sizes: &[NgramSize] = if characters_count < self.long_text_minlen {
            &self.short_text_ngram_sizes
        } else {
            &self.long_text_ngram_sizes
        };
        debug_assert!(!ngram_sizes.is_empty());

        /* if characters_count < ngram_length_range.start {
            return filtered_languages
                .into_iter()
                .map(|l| (l, f64::NEG_INFINITY))
                .collect();
        } */

        let wordgrams_enabled = *ngram_sizes.last().unwrap_safe_unchecked() == NgramSize::Word;
        if wordgrams_enabled {
            ngram_sizes = ngram_sizes.get_safe_unchecked(..ngram_sizes.len() - 1);
        }

        let mut probabilities = slang_arr_default::<(f64, usize)>();

        for &ngram_size in ngram_sizes {
            Self::probabilities_ngrams(
                self.models_storage,
                words.iter().map(|wd| wd.buf.as_ref()),
                &filtered_languages,
                ngram_size,
                &mut probabilities,
            );
        }

        if wordgrams_enabled {
            Self::probabilities_languages_wordgrams(
                self.models_storage,
                words.iter().map(|wd| wd.buf.iter().collect::<String>()),
                &filtered_languages,
                &mut probabilities,
            );
        }

        let mut probabilities_mean = Self::probabilities_mean(probabilities, filtered_languages);

        probabilities_mean.sort_unstable_by(order_by_probability_and_lang);
        /* println!(
            "res {:?}",
            &probabilities_mean[..probabilities_mean.len().min(5)]
        ); */

        ProbabilitiesExtra {
            probabilities: probabilities_mean,
            words,
        }
    }

    /// Returns probabilities for the provided text.
    /// Each value is a logarithmic probability between a negative infinity and 0.0.
    ///
    /// Result is sorted by probabilities in a descending order.
    ///
    /// If only a single language is identified by `alphabet_detector`,
    /// the value 0.0 will be returned.
    #[inline]
    pub fn probabilities(&self, text: &str) -> Vec<(ScriptLanguage, f64)> {
        self.probabilities_extra(text).probabilities
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
    pub fn detect_top_one_or_none(
        &self,
        text: &str,
        minimum_distance: f64,
    ) -> Option<ScriptLanguage> {
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

    /// Detects a top one language of the provided text.
    /// If multiple languages are covered by reorder distance (result of `reorder_distance_compute`),
    /// reorders by total speakers of these languages.
    ///
    /// Reorder distance is a distance between logarithmic probabilities, must be >= 0.0.
    ///
    /// [`None`] is returned only when `probabilities` is empty.
    pub fn detect_top_one_reordered_custom<F>(
        &self,
        text: &str,
        reorder_distance_compute: F,
    ) -> Option<ScriptLanguage>
    where
        F: FnOnce(Vec<Word<Vec<char>>>) -> f64,
    {
        let ProbabilitiesExtra {
            mut probabilities,
            words,
        } = self.probabilities_extra(text);

        let (_first_language, first_probability) = *probabilities.first()?;

        let reorder_distance = reorder_distance_compute(words);
        debug_assert!(reorder_distance >= 0.0, "Reorder distance must be >= 0.0");

        let reorder_probability = first_probability - reorder_distance;
        let lim = probabilities
            .iter()
            .position(|(_, p)| *p < reorder_probability)
            .unwrap_or(probabilities.len());
        probabilities.truncate(lim);
        probabilities.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        probabilities.first().map(|(l, _)| *l)
    }

    /// Detects a top one language of the provided text.
    /// If multiple languages are covered by the reorder formula,
    /// reorders by total speakers of these languages.
    /// More sutable if you need better detection of common (more popular) languages.
    ///
    /// [`None`] is returned only when `probabilities` is empty.
    pub fn detect_top_one_reordered(&self, text: &str) -> Option<ScriptLanguage> {
        self.detect_top_one_reordered_custom(
            text,
            #[inline]
            |words: Vec<Word<Vec<char>>>| {
                let characters_bytes_count: usize = words
                    .iter()
                    .flat_map(|wd| wd.buf.iter().map(|c| c.len_utf8()))
                    .sum();

                1.35 / (characters_bytes_count + words.len().pow(3) - 1) as f64
            },
        )
    }

    /// Detects a top one language of the provided text.
    /// More sutable if you need better detection of rare (less popular) languages.
    ///
    /// [`None`] is returned only when `probabilities` is empty.
    pub fn detect_top_one_raw(&self, text: &str) -> Option<ScriptLanguage> {
        self.detect_top_one_reordered_custom(
            text,
            #[inline]
            |_| 0.0,
        )
    }

    #[inline(always)]
    pub fn detect_top_one(&self, text: &str, reorder: bool) -> Option<ScriptLanguage> {
        if reorder {
            self.detect_top_one_reordered(text)
        } else {
            self.detect_top_one_raw(text)
        }
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
    if first_probability == 0.0 {
        let zeroes = probabilities
            .iter()
            .position(|(_, p)| *p != 0.0)
            .unwrap_or(probabilities.len());
        probabilities.truncate(zeroes);
    }

    if first_probability == 0.0 || first_probability == f64::NEG_INFINITY {
        let len = probabilities.len() as f64;
        probabilities.iter_mut().for_each(|(_, p)| *p = 1.0 / len);

        return;
    }

    let mut denominator: f64 = 0.0;
    probabilities.iter_mut().for_each(|(_, p)| {
        *p = p.exp();
        denominator += *p;
    });

    if denominator == 0.0 {
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

#[derive(Default, Debug, Clone)]
struct ProbabilitiesExtra {
    probabilities: Vec<(ScriptLanguage, f64)>,
    words: Vec<Word<Vec<char>>>,
}
