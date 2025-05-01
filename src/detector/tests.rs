use super::{model::Model, *};
use crate::ScriptLanguage::*;
use ::std::{collections::HashSet, sync::LazyLock};
use ahash::AHashMap;
use compact_str::CompactString;
use float_cmp::approx_eq;
use rstest::*;

fn create_mock_model(
    ngrams_model: [AHashMap<&'static str, f64>; NGRAM_MAX_LEN],
    wordgrams_model: AHashMap<&'static str, f64>,
) -> Model {
    let ngrams = ngrams_model.map(|model| {
        model
            .into_iter()
            .map(|(k, v)| (NgramString::try_from_str(k).unwrap(), v.ln()))
            .collect()
    });
    let wordgrams = wordgrams_model
        .into_iter()
        .map(|(k, v)| (CompactString::from(k), v.ln()))
        .collect();
    Model::new(ngrams, wordgrams)
}

fn round_to_two_decimal_places(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

const ENGLISH_UNIGRAMS_COUNT: f64 = 7.0;
fn model_for_english() -> Model {
    create_mock_model(
        [
            ahashmap!(
                "a" => 0.01,
                "l" => 0.02,
                "t" => 0.03,
                "e" => 0.04,
                "r" => 0.05,
                "o" => 1.0,
                "k" => 1.0,
            ),
            ahashmap!(
                "al" => 0.11,
                "lt" => 0.12,
                "te" => 0.13,
                "er" => 0.14,
            ),
            ahashmap!(
                "alt" => 0.19,
                "lte" => 0.2,
                "ter" => 0.21,
            ),
            ahashmap!(
                "alte" => 0.25,
                "lter" => 0.26,
            ),
            ahashmap!(
                "alter" => 0.29,
            ),
        ],
        ahashmap!(
            "alter" => 0.29,
        ),
    )
}

const GERMAN_UNIGRAMS_COUNT: f64 = 6.0;
fn model_for_german() -> Model {
    create_mock_model(
        [
            ahashmap!(
                "a" => 0.06,
                "l" => 0.07,
                "t" => 0.08,
                "e" => 0.09,
                "r" => 0.1,
                "o" => 1.0,
            ),
            ahashmap!(
                "al" => 0.15,
                "lt" => 0.16,
                "te" => 0.17,
                "er" => 0.18,
            ),
            ahashmap!(
                "alt" => 0.22,
                "lte" => 0.23,
                "ter" => 0.24,
            ),
            ahashmap!(
                "alte" => 0.27,
                "lter" => 0.28,
            ),
            ahashmap!("alter" => 0.3),
        ],
        ahashmap!("alter" => 0.3),
    )
}

static MOCK_MODELS_ENGLISH_AND_GERMAN: LazyLock<ModelsStorage> = LazyLock::new(|| {
    let models_storage: ModelsStorage = Default::default();
    *models_storage
        .0
        .get_safe_unchecked(English as usize)
        .write()
        .unwrap() = model_for_english();
    *models_storage
        .0
        .get_safe_unchecked(German as usize)
        .write()
        .unwrap() = model_for_german();
    models_storage
});

static MODELS_ALL_LANGUAGES_PRELOADED: LazyLock<ModelsStorage> = LazyLock::new(|| {
    ModelsStorage::preloaded::<ahash::RandomState>(ScriptLanguage::all().collect())
});

#[rstest(
    language,
    ngram,
    expected_probability,
    case(English, "a", 0.01),
    case(English, "lt", 0.12),
    case(English, "ter", 0.21),
    case(English, "alte", 0.25),
    case(English, "alter", 0.29),
    case(German, "t", 0.08),
    case(German, "er", 0.18),
    case(German, "alt", 0.22),
    case(German, "lter", 0.28),
    case(German, "alter", 0.3)
)]
fn test_mock_model_ngram_lookup(language: ScriptLanguage, ngram: &str, expected_probability: f64) {
    let ngram_length = ngram.chars().count();
    // mock_detector_for_english_and_german
    // .load_language_models_by_ngram_len(ngram_length, &ahashset!(language));

    let language_model_lock = MOCK_MODELS_ENGLISH_AND_GERMAN
        .0
        .get_safe_unchecked(language as usize)
        .read()
        .unwrap();

    let probability = language_model_lock.ngrams[ngram_length - 1]
        .get(ngram)
        .copied()
        .unwrap_or(f64::NEG_INFINITY);

    let expected_probability = expected_probability.ln();

    assert_eq!(
        probability, expected_probability,
        "expected probability {} for language '{:?}' and ngram '{}', got {}",
        expected_probability, language, ngram, probability
    );
}

#[rstest(
    ngrams,
    expected_ngrams_sum,
    expected_ngrams_cnt,
    case(
        vec!["a", "l", "t", "e", "r"],
        0.01_f64.ln() + 0.02_f64.ln() + 0.03_f64.ln() + 0.04_f64.ln() + 0.05_f64.ln(),
        5
    ),
    case(
        // last one is unknown trigram
        vec!["alt", "lte", "tez"],
        0.19_f64.ln() + 0.2_f64.ln() + (1_f64 / ENGLISH_UNIGRAMS_COUNT).ln(),
        2
    ),
    case(
        // unknown fivegram
        vec!["aquas"],
        (1_f64 / ENGLISH_UNIGRAMS_COUNT).ln(),
        0
    ),
    case(
        // English only unigram
        vec!["k"],
        1.0_f64.ln(),
        1
    )
)]
fn test_mock_ngrams_sum_cnt(
    ngrams: Vec<&'static str>,
    expected_ngrams_sum: f64,
    expected_ngrams_cnt: usize,
) {
    let detector = Detector::new(
        DetectorConfig::with_languages(ahashset!(English)),
        &MOCK_MODELS_ENGLISH_AND_GERMAN,
    );
    let (ngrams_sum, ngrams_cnt) = detector.ngrams_sum_cnt(
        English,
        ngrams.iter().copied(),
        NgramsSize::from(ngrams[0].chars().count() - 1),
    );

    assert!(
        approx_eq!(f64, ngrams_sum, expected_ngrams_sum, ulps = 1),
        "expected sum {} for language '{:?}' and ngrams {:?}, got {}",
        expected_ngrams_sum,
        English,
        ngrams,
        ngrams_sum
    );

    assert_eq!(
        ngrams_cnt, expected_ngrams_cnt,
        "expected cnt {} for language '{:?}' and ngrams {:?}, got {}",
        expected_ngrams_cnt, English, ngrams, ngrams_cnt
    );
}

#[rstest(
    ngrams,
    expected_probabilities,
    case::unigram_model(
        vec!["a", "l", "t", "e", "r"],
        ahashmap!(
            English => 0.01_f64.ln() + 0.02_f64.ln() + 0.03_f64.ln() + 0.04_f64.ln() + 0.05_f64.ln(),
            German => 0.06_f64.ln() + 0.07_f64.ln() + 0.08_f64.ln() + 0.09_f64.ln() + 0.1_f64.ln()
        )
    ),
    case::trigram_model(
        vec!["alt", "lte", "ter", "wxy"],
        ahashmap!(
            English => 0.19_f64.ln() + 0.2_f64.ln() + 0.21_f64.ln() + (1_f64 / ENGLISH_UNIGRAMS_COUNT).ln(),
            German => 0.22_f64.ln() + 0.23_f64.ln() + 0.24_f64.ln() + (1_f64 / GERMAN_UNIGRAMS_COUNT).ln()
        )
    ),
    case::quadrigram_model(
        vec!["alte", "lter", "wxyz"],
        ahashmap!(
            English => 0.25_f64.ln() + 0.26_f64.ln() + (1_f64 / ENGLISH_UNIGRAMS_COUNT).ln(),
            German => 0.27_f64.ln() + 0.28_f64.ln() + (1_f64 / GERMAN_UNIGRAMS_COUNT).ln()
        )
    )
)]
fn test_mock_probabilities_languages_ngrams(
    ngrams: Vec<&'static str>,
    expected_probabilities: AHashMap<ScriptLanguage, f64>,
) {
    let languages: AHashSet<ScriptLanguage> = ahashset!(English, German);
    let detector = Detector::new(
        DetectorConfig::with_languages(languages.clone().into()),
        &MOCK_MODELS_ENGLISH_AND_GERMAN,
    );
    let probabilities = detector.probabilities_languages_ngrams(
        ngrams.iter().copied(),
        &languages,
        NgramsSize::from(ngrams[0].chars().count() - 1),
    );

    for (language, (probability, _cnt)) in probabilities.into_iter().enumerate() {
        if probability.is_zero() {
            continue;
        }
        let language = ScriptLanguage::transmute_from_usize(language);
        let expected_probability = expected_probabilities[&language];

        assert!(
            approx_eq!(f64, probability, expected_probability, ulps = 1),
            "expected probability {} for language '{:?}', got {}",
            expected_probability,
            language,
            probability
        );
    }
}

#[rstest(
    text,
    expected_probabilities,
    case::language_detected_by_rules("groß", vec![(German, 1.0)]),
    case::known_ngrams("Alter", vec![(German, 0.61), (English, 0.39)]),
    case::english_only_ngrams("k", vec![(English, 1.0)]),
    case::unique_ngrams("o", vec![(English, 0.5), (German, 0.5)]),
    case::unknown_ngrams("проарплап", vec![]),
)]
fn test_mock_probabilities_relative(
    text: &str,
    expected_probabilities: Vec<(ScriptLanguage, f64)>,
) {
    let detector = Detector::new(
        DetectorConfig::with_languages(ahashset!(English, German)),
        &MOCK_MODELS_ENGLISH_AND_GERMAN,
    );

    let mut probabilities = detector.probabilities_relative(text);
    probabilities
        .iter_mut()
        .for_each(|(_, p)| *p = round_to_two_decimal_places(*p));

    assert_eq!(probabilities, expected_probabilities);
}

#[rstest(
    text,
    expected_probabilities,
    case::script_no_models("ꨕ", vec![(ChamEastern, 0.5), (ChamWestern, 0.5)]),
)]
fn test_mock_probabilities_relative_no_filter(
    text: &str,
    expected_probabilities: Vec<(ScriptLanguage, f64)>,
) {
    let detector = Detector::new(
        DetectorConfig::new_all_languages(),
        &MOCK_MODELS_ENGLISH_AND_GERMAN,
    );

    let mut probabilities = detector.probabilities_relative(text);
    probabilities
        .iter_mut()
        .for_each(|(_, p)| *p = round_to_two_decimal_places(*p));

    assert_eq!(probabilities, expected_probabilities);
}

#[rstest(
    word,
    expected_language,
    case("Alter", Some(German)),
    case("проарплап", None)
)]
fn test_mock_detect_top_one(word: &str, expected_language: Option<ScriptLanguage>) {
    let detector = Detector::new(
        DetectorConfig::with_languages(ahashset!(English, German)),
        &MOCK_MODELS_ENGLISH_AND_GERMAN,
    );
    let detected_language = detector.detect_top_one(word, 0.0);
    assert_eq!(detected_language, expected_language);
}

/* #[rstest]
fn test_detect_multiple_for_empty_string() {
    let detector = LanguageDetector::new(
        LanguageDetectorConfig::new_all_languages(),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    assert!(detector.detect_multiple("").is_empty());
}

#[rstest(
    sentence,
    expected_word_count,
    expected_language,
    case::english_1(
        "I'm really not sure whether multi-language detection is a good idea.",
        11,
        English
    ),
    case::english_2("I'm frightened! 🙈", 3, English),
    case::kazakh("V төзімділік спорт", 3, Kazakh)
)]
fn test_detect_multiple_with_one_language(
    sentence: &str,
    expected_word_count: usize,
    expected_language: ScriptLanguage,
) {
    let detector = LanguageDetector::new(
        LanguageDetectorConfig::new_all_languages(),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    let results = detector.detect_multiple(sentence);
    assert_eq!(results.len(), 1);

    let result = &results[0];
    let substring = &sentence[result.start_index()..result.end_index()];
    assert_eq!(substring, sentence);
    assert_eq!(result.word_count, expected_word_count);
    assert_eq!(result.language(), expected_language);
}

#[rstest(
    sentence,
    expected_first_substring,
    expected_first_word_count,
    expected_first_language,
    expected_second_substring,
    expected_second_word_count,
    expected_second_language,
    case::english_german(
        "  He   turned around and asked: \"Entschuldigen Sie, sprechen Sie Deutsch?\"",
        "  He   turned around and asked: ",
        5,
        English,
        "\"Entschuldigen Sie, sprechen Sie Deutsch?\"",
        5,
        German
    ),
    case::chinese_english(
        "上海大学是一个好大学. It is such a great university.",
        "上海大学是一个好大学. ",
        10,
        ChineseSimplified,
        "It is such a great university.",
        6,
        English
    ),
    case::english_russian(
        "English German French - Английский язык",
        "English German French - ",
        4,
        English,
        "Английский язык",
        2,
        Russian
    )
)]
fn test_detect_multiple_with_two_languages(
    sentence: &str,
    expected_first_substring: &str,
    expected_first_word_count: usize,
    expected_first_language: ScriptLanguage,
    expected_second_substring: &str,
    expected_second_word_count: usize,
    expected_second_language: ScriptLanguage,
) {
    let detector = LanguageDetector::new(
        LanguageDetectorConfig::new_all_languages(),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    let results = detector.detect_multiple(sentence);
    assert_eq!(results.len(), 2);

    let first_result = &results[0];
    let first_substring = &sentence[first_result.start_index()..first_result.end_index()];
    assert_eq!(first_substring, expected_first_substring);
    assert_eq!(first_result.word_count, expected_first_word_count);
    assert_eq!(first_result.language(), expected_first_language);

    let second_result = &results[1];
    let second_substring = &sentence[second_result.start_index()..second_result.end_index()];
    assert_eq!(second_substring, expected_second_substring);
    assert_eq!(second_result.word_count, expected_second_word_count);
    assert_eq!(second_result.language(), expected_second_language);
}

#[rstest(
    sentence,
    expected_first_substring,
    expected_first_word_count,
    expected_first_language,
    expected_second_substring,
    expected_second_word_count,
    expected_second_language,
    expected_third_substring,
    expected_third_word_count,
    expected_third_language,
    case::french_german_english(
        "Parlez-vous français? Ich spreche Französisch nur ein bisschen. A little bit is better than nothing.",
        "Parlez-vous français? ",
        2,
        French,
        "Ich spreche Französisch nur ein bisschen. ",
        6,
        German,
        "A little bit is better than nothing.",
        7,
        English
    ),
    /* case::polish_german_english(
        "Płaszczowo-rurowe wymienniki ciepła Uszczelkowe der blaue himmel über berlin 中文 the quick brown fox jumps over the lazy dog",
        "Płaszczowo-rurowe wymienniki ciepła Uszczelkowe ",
        4,
        Polish,
        "der blaue himmel über berlin 中文 ",
        7,
        German,
        "the quick brown fox jumps over the lazy dog",
        9,
        English
    ), */
)]
fn test_detect_multiple_with_three_languages(
    sentence: &str,
    expected_first_substring: &str,
    expected_first_word_count: usize,
    expected_first_language: ScriptLanguage,
    expected_second_substring: &str,
    expected_second_word_count: usize,
    expected_second_language: ScriptLanguage,
    expected_third_substring: &str,
    expected_third_word_count: usize,
    expected_third_language: ScriptLanguage,
) {
    let detector = LanguageDetector::new(
        LanguageDetectorConfig::new_all_languages(),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    let results = detector.detect_multiple(sentence);
    assert_eq!(results.len(), 3, "{} {:?}", sentence, results);

    let first_result = &results[0];
    let first_substring = &sentence[first_result.start_index()..first_result.end_index()];
    assert_eq!(first_substring, expected_first_substring);
    assert_eq!(first_result.word_count, expected_first_word_count);
    assert_eq!(first_result.language(), expected_first_language);

    let second_result = &results[1];
    let second_substring = &sentence[second_result.start_index()..second_result.end_index()];
    assert_eq!(second_substring, expected_second_substring);
    assert_eq!(second_result.word_count, expected_second_word_count);
    assert_eq!(second_result.language(), expected_second_language);

    let third_result = &results[2];
    let third_substring = &sentence[third_result.start_index()..third_result.end_index()];
    assert_eq!(third_substring, expected_third_substring);
    assert_eq!(third_result.word_count, expected_third_word_count);
    assert_eq!(third_result.language(), expected_third_language);
}*/

/* #[rstest(
    sentence,
    expected_first_substring,
    expected_first_word_count,
    expected_first_language,
    expected_second_substring,
    expected_second_word_count,
    expected_second_language,
    expected_third_substring,
    expected_third_word_count,
    expected_third_language,
    expected_fourth_substring,
    expected_fourth_word_count,
    expected_fourth_language,
    case::polish_german_chinese_english(
        "Płaszczowo-rurowe wymienniki ciepła Uszczelkowe der blaue himmel über berlin 中文 the quick brown fox jumps over the lazy dog",
        "Płaszczowo-rurowe wymienniki ciepła Uszczelkowe ",
        4,
        Polish,
        "der blaue himmel über berlin ",
        5,
        German,
        "中文 ",
        2,
        Chinese,
        "the quick brown fox jumps over the lazy dog",
        9,
        English
    )
)]
fn test_detect_multiple_with_four_languages(
    sentence: &str,
    expected_first_substring: &str,
    expected_first_word_count: usize,
    expected_first_language: Language,
    expected_second_substring: &str,
    expected_second_word_count: usize,
    expected_second_language: Language,
    expected_third_substring: &str,
    expected_third_word_count: usize,
    expected_third_language: Language,
    expected_fourth_substring: &str,
    expected_fourth_word_count: usize,
    expected_fourth_language: Language,
) {
    let results = DETECTOR_ALL_LANGUAGES.detect_multiple(sentence);
    assert_eq!(results.len(), 4, "{:?}", results);

    let first_result = &results[0];
    let first_substring = &sentence[first_result.start_index()..first_result.end_index()];
    assert_eq!(first_substring, expected_first_substring);
    assert_eq!(first_result.word_count, expected_first_word_count);
    assert_eq!(first_result.language(), expected_first_language);

    let second_result = &results[1];
    let second_substring = &sentence[second_result.start_index()..second_result.end_index()];
    assert_eq!(second_substring, expected_second_substring);
    assert_eq!(second_result.word_count, expected_second_word_count);
    assert_eq!(second_result.language(), expected_second_language);

    let third_result = &results[2];
    let third_substring = &sentence[third_result.start_index()..third_result.end_index()];
    assert_eq!(third_substring, expected_third_substring);
    assert_eq!(third_result.word_count, expected_third_word_count);
    assert_eq!(third_result.language(), expected_third_language);

    let fourth_result = &results[3];
    let fourth_substring = &sentence[fourth_result.start_index()..fourth_result.end_index()];
    assert_eq!(fourth_substring, expected_fourth_substring);
    assert_eq!(fourth_result.word_count, expected_fourth_word_count);
    assert_eq!(fourth_result.language(), expected_fourth_language);
} */

#[rstest(
    expected_language,
    text,
    case(Kazakh, "нормаланбайды"),
    case(Kazakh, "нормаланбайды I"),
    case(Kazakh, "Балаларды жүзуге үй-рету бассейнінің үй-жайы"),
    case(English, "I know you әлем"),
    case(ChineseMandarinSimplified, "经济"),
    case(ChineseMandarinTraditional, "經濟"),
    case::kanji(Japanese, "経済"),
    case::kanji2(Japanese, "自動販売機"),
    // case(Arabic, "كيف حالك؟")
)]
fn test_detect_top_one(expected_language: ScriptLanguage, text: &str) {
    let detector = Detector::new(
        DetectorConfig::new_all_languages(),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    assert_eq!(detector.detect_top_one(text, 0.0), Some(expected_language));
}

#[rstest(text, languages,
    case(
        "ام وی با نیکی میناج تیزر داشت؟؟؟؟؟؟ i vote for bts ( _ ) as the _ via ( _ )",
        ahashset!(English, Urdu)
    ),
    case(
        "Az elmúlt hétvégén 12-re emelkedett az elhunyt koronavírus-fertőzöttek száma Szlovákiában. Mindegyik szociális otthon dolgozóját letesztelik, Matovič szerint az ingázóknak még várniuk kellene a teszteléssel",
        ahashset!(Hungarian, Slovak)
    )
)]
fn test_detect_top_one_is_deterministic(text: &str, languages: AHashSet<ScriptLanguage>) {
    let detector_config = DetectorConfig::with_languages(languages.clone().into());
    let detector = Detector::new(detector_config, &MODELS_ALL_LANGUAGES_PRELOADED);

    let mut detected_languages = AHashSet::new();
    for _ in 0..100 {
        let language = detector.detect_top_one(text, 0.0);
        detected_languages.insert(language.unwrap());
    }
    assert_eq!(
        detected_languages.len(),
        1,
        "language detector is non-deterministic for languages {:?}",
        languages
    );
}

/* #[rstest(
    expected_language,
    text,
    languages,
    case::arab(Arabic, "والموضوع", ahashset![English, Arabic]),
)]
fn test_detect_top_one_with_languages(
    expected_language: ScriptLanguage,
    text: &str,
    languages: AHashSet<ScriptLanguage>,
) {
    let detector = Detector::new(
        DetectorConfig::with_languages(languages.into()),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    let language = detector.detect_top_one(text, 0.0).unwrap();
    assert_eq!(language, expected_language);
} */

/* #[should_panic]
#[rstest(text,
    case("kejurnas iii пїѕ aa boxer cup iii пїѕ bertempat di bandung jumlah peserta petarung dari daerah provinsi jawa barat dki jakarta jawa timur sumatera utara sumatera barat nusa tenggara barat bali kalimantan barat"),
)]
fn assert_language_filtering_with_rules_text_panics(
    text: &str,
) {
    let words = split_text_into_words(text);

    let filtered_languages =
        LanguageDetector::process_words(&words, &DETECTOR_ALL_LANGUAGES.languages);

    /* let words_count_half = (words.len() as f64) * 0.5;
    let filtered_languages = DETECTOR_ALL_LANGUAGES.filter_languages_by_rules(
        &words,
        // &DETECTOR_ALL_LANGUAGES.languages,
        words_count_half,
        // alps,
        filtered_languages,
    ); */
} */

#[rstest(invalid_str, case(""), case(" \n  \t;"), case("3<856%)§"))]
fn test_strings_without_letters(invalid_str: &str) {
    let detector = Detector::new(
        DetectorConfig::new_all_languages(),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    assert_eq!(detector.detect_top_one(invalid_str, 0.0), None);
}

#[test]
fn test_max_trigrams_mode() {
    let detector_config = DetectorConfig::with_languages(ahashset!(English, German)).max_trigrams();
    let detector = Detector::new(detector_config, &MODELS_ALL_LANGUAGES_PRELOADED);

    assert!(detector.detect_top_one("bed", 0.0).is_some());
    assert!(detector.detect_top_one("be", 0.0).is_some());
    assert!(detector.detect_top_one("b", 0.0).is_some());

    assert!(detector.detect_top_one("", 0.0).is_none());
}

#[test]
fn test_change_langs() {
    let config_new = DetectorConfig::new_all_languages();
    let config_hash = config_new.copy_with_languages(HashSet::new());
    let _config_ahash = config_hash.languages(ahashset!(English));
}
