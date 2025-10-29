use super::{builder::DetectorBuilder, *};
use crate::{model::Model, NgramSize, ScriptLanguage::*};
use ::std::sync::LazyLock;
use ahash::AHashMap;
use float_cmp::approx_eq;
use rstest::*;
use strum::EnumCount;

fn create_mock_model(ngrams_model: [AHashMap<&'static str, f64>; NgramSize::COUNT]) -> Model {
    let ngrams = ngrams_model.map(|model| {
        model
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.ln()))
            .collect()
    });
    Model::new_mock(ngrams)
}

fn round_to_two_decimal_places(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn model_for_english() -> Model {
    create_mock_model([
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
        ahashmap!(
            "alter" => 0.29,
        ),
    ])
}

fn model_for_german() -> Model {
    create_mock_model([
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
        ahashmap!("alter" => 0.3),
    ])
}

static MOCK_MODELS_ENGLISH_AND_GERMAN: LazyLock<ModelsStorage> = LazyLock::new(|| {
    let models_storage =
        ModelsStorage::from_models([(English, model_for_english()), (German, model_for_german())]);
    models_storage
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

    let probability = MOCK_MODELS_ENGLISH_AND_GERMAN.ngrams[ngram_length - 1]
        .get(ngram)
        .and_then(|v| {
            v.iter().find_map(|ArchivedTuple2(l, p)| {
                if ScriptLanguage::transmute_from_usize(l.to_native() as usize) == language {
                    Some(p.to_native())
                } else {
                    None
                }
            })
        })
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
            English => 0.19_f64.ln() + 0.2_f64.ln() + 0.21_f64.ln(),
            German => 0.22_f64.ln() + 0.23_f64.ln() + 0.24_f64.ln()
        )
    ),
    case::quadrigram_model(
        vec!["alte", "lter", "wxyz"],
        ahashmap!(
            English => 0.25_f64.ln() + 0.26_f64.ln(),
            German => 0.27_f64.ln() + 0.28_f64.ln()
        )
    ),
    case::english_only(
        vec!["k"],
        ahashmap!(English => 1.0_f64.ln())
    )
)]
fn test_mock_probabilities_languages_ngrams(
    ngrams: Vec<&'static str>,
    expected_probabilities: AHashMap<ScriptLanguage, f64>,
) {
    let languages: FxHashSet<ScriptLanguage> = FxHashSet::from_iter([English, German]);

    let mut probabilities = slang_arr_default::<(f64, usize)>();
    Detector::probabilities_languages_ngrams(
        &MOCK_MODELS_ENGLISH_AND_GERMAN,
        NgramSize::from(ngrams[0].chars().count() - 1),
        ngrams.iter().copied(),
        &languages,
        &mut probabilities,
    );

    for (language, (probability, cnt)) in probabilities.into_iter().enumerate() {
        if cnt == 0 {
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
    // can return different result if `wordgram_min_probability` was changed in another detector (because it's static)
    case::english_only_ngrams("k", vec![(English, 1.0)]),
    case::unique_ngrams("o", vec![(English, 0.5), (German, 0.5)]),
    case::unknown_ngrams("проарплап", vec![]),
)]
fn test_mock_probabilities_relative(
    text: &str,
    expected_probabilities: Vec<(ScriptLanguage, f64)>,
) {
    let detector = DetectorBuilder::new(&MOCK_MODELS_ENGLISH_AND_GERMAN)
        .languages(ahashset!(English, German))
        .build();

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
    let detector = DetectorBuilder::new(&MOCK_MODELS_ENGLISH_AND_GERMAN).build();

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
fn test_mock_detect_top_one_raw(word: &str, expected_language: Option<ScriptLanguage>) {
    let detector = DetectorBuilder::new(&MOCK_MODELS_ENGLISH_AND_GERMAN)
        .languages(ahashset!(English, German))
        .build();
    let detected_language = detector.detect_top_one_raw(word);
    assert_eq!(detected_language, expected_language);
}

#[rstest(
    word,
    expected_language,
    case::script_no_models("ꨕ", Some(ChamEastern))
)]
fn test_mock_detect_top_one_raw_no_filter(word: &str, expected_language: Option<ScriptLanguage>) {
    let detector = DetectorBuilder::new(&MOCK_MODELS_ENGLISH_AND_GERMAN).build();
    let detected_language = detector.detect_top_one_raw(word);
    assert_eq!(detected_language, expected_language);
}

#[rstest(word, expected_language, case::script_no_models("ꨕ", None))]
fn test_mock_detect_top_one_or_none_no_filter(
    word: &str,
    expected_language: Option<ScriptLanguage>,
) {
    let detector = DetectorBuilder::new(&MOCK_MODELS_ENGLISH_AND_GERMAN).build();
    let detected_language = detector.detect_top_one_or_none(word, 0.0);
    assert_eq!(detected_language, expected_language);
}
