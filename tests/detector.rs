use ::std::sync::LazyLock;
use ahash::AHashSet;
use langram::{ahashset, DetectorBuilder, ModelsStorage, ScriptLanguage, ScriptLanguage::*};
use rstest::*;

static MODELS_ALL_LANGUAGES_PRELOADED: LazyLock<ModelsStorage> = LazyLock::new(|| {
    ModelsStorage::preloaded::<ahash::RandomState>(ScriptLanguage::all().collect())
});

#[rstest(
    expected_language,
    text,
    case(Kazakh, "нормаланбайды"),
    case(Kazakh, "нормаланбайды I"),
    case(Kazakh, "Балаларды жүзуге үй-рету бассейнінің үй-жайы"),
    case(English, "I know you әлем"),
    case(English, "love әлем"),
    case::unknown_words(
        English,
        "A vibrator, sometimes described as a massager, is a sex toy that is used on the body to produce pleasurable sexual stimulation"
    ),
    case::mixed(English, "¿que? Hello, how are you? I am well, thank you."),
    // case::mixed(English, "¿cómo estás? Hello, how are you? I am well, thank you."),
    case(NorwegianBokmal, "Et Sprang i Tiden"),
    case(NorwegianBokmal, "Løvenes konge"),
    case(NorwegianBokmal, "Det kommer båter"),
    case(NorwegianBokmal, "Om hester og menn"),
    case(NorwegianBokmal, "Fødeavdelingen"),
    case(NorwegianBokmal, "Fabeldyr: Grindelwalds Forbrytelser"),
    case(NorwegianNynorsk, "Kor gamal er ho?"),
    case(NorwegianNynorsk, "Det er heilt topp"),
    case(NorwegianNynorsk, "Skal vi vere vener?"),
    // case(Arabic, "كيف حالك؟"),

    // words
    case(AlbanianTosk, "hashemidëve"),
    case(AzerbaijaniNorth, "məhərrəm"),
    case(Belarusian, "павінен"),
    case(Belarusian, "раскрывае"),
    case(Bengali, "জানাতে"),
    case(Bulgarian, "довършат"),
    case(Bulgarian, "плаваща"),
    case(Catalan, "contradicció"),
    case(Catalan, "només"),
    case(Catalan, "pràctiques"),
    case(Catalan, "substituïts"),
    case(ChineseMandarinTraditional, "經濟"),
    case(Croatian, "nađete"),
    case(Croatian, "prihvaćanju"),
    case(Czech, "jeďte"),
    case(Czech, "navržen"),
    case(Czech, "rozdělit"),
    case(Czech, "rtuť"),
    case(Czech, "subjektů"),
    case(Czech, "zaručen"),
    case(Czech, "zkouškou"),
    case(Danish, "direktør"),
    case(Danish, "indebærer"),
    case(Danish, "måned"),
    case(English, "house"),
    case(Esperanto, "apenaŭ"),
    case(Estonian, "päralt"),
    case(Estonian, "tõeliseks"),
    case(French, "contrôle"),
    case(French, "façonnage"),
    case(French, "forêt"),
    case(French, "où"),
    case(French, "succèdent"),
    case(German, "höher"),
    case(German, "überrascht"),
    case(Hebrew, "בתחרויות"),
    case(Icelandic, "minjaverðir"),
    case(Italian, "venerdì"),
    case(Japanese, "東京"),
    case(Japanese, "経済"),
    case(Kazakh, "әлем"),
    case(Kazakh, "оның"),
    case(Kazakh, "шаруашылығы"),
    case(Latvian, "aizklātā"),
    case(Latvian, "blaķene"),
    case(Latvian, "ceļojumiem"),
    case(Latvian, "labāk"),
    case(Latvian, "nebūtu"),
    case(Latvian, "numuriņu"),
    case(Latvian, "palīdzi"),
    case(Latvian, "sistēmas"),
    case(Latvian, "teoloģiska"),
    case(Latvian, "viņiem"),
    case(Lithuanian, "įrengus"),
    case(Lithuanian, "mergelės"),
    case(Lithuanian, "nebūsime"),
    case(Lithuanian, "slegiamų"),
    case(Macedonian, "затоплување"),
    case(Macedonian, "ѕидови"),
    case(Macedonian, "набљудувач"),
    case(Macedonian, "џамиите"),
    case(Marathi, "मिळते"),
    case(MongolianHalh, "дөхөж"),
    case(MongolianHalh, "үндсэн"),
    case(Polish, "budowę"),
    case(Polish, "groźne"),
    case(Polish, "kradzieżami"),
    case(Polish, "mniejszości"),
    case(Polish, "państwowych"),
    case(Polish, "zmieniły"),
    case(Portuguese, "visão"),
    case(Romanian, "afişate"),
    case(Romanian, "înviat"),
    case(Romanian, "pregătire"),
    case(Russian, "огнём"),
    case(Russian, "сопротивление"),
    case(Russian, "этот"),
    case(Spanish, "¿que?"),
    case(Spanish, "años"),
    case(Ukrainian, "пристрої"),
    case(Vietnamese, "chỉnh"),
    case(Vietnamese, "chọn"),
    case(Vietnamese, "của"),
    case(Vietnamese, "cũng"),
    case(Vietnamese, "dụng"),
    case(Vietnamese, "kẽm"),
    case(Vietnamese, "lẻn"),
    case(Vietnamese, "mỹ"),
    case(Vietnamese, "nhẹn"),
    case(Vietnamese, "sỏi"),
    case(Vietnamese, "trĩ"),
    case(Yoruba, "ṣaaju"),
    // case(Hawaiian, "pu'u'ō'ō"),
    // case(Macedonian, "ректасцензија"),
    // case(Portuguese, "catedráticos"),
    // case(Portuguese, "música"),
    // case(Portuguese, "política"),
    // case(Slovak, "rozohňuje"),
    // case(Vietnamese, "ravị"),
)]
fn test_detect_top_one_raw(expected_language: ScriptLanguage, text: &str) {
    let detector = DetectorBuilder::new(&MODELS_ALL_LANGUAGES_PRELOADED).build();
    assert_eq!(
        detector.detect_top_one_raw(text),
        Some(expected_language),
        "detect_top_one_raw {}",
        text
    );

    assert_eq!(
        detector.detect_top_one_or_none(text, 0.0),
        Some(expected_language),
        "detect_top_one_or_none {}",
        text
    );

    // extra check, same as `test_detect_top_one_reordered` below
    assert_eq!(
        detector.detect_top_one_reordered(text),
        Some(expected_language),
        "detect_top_one_reordered {}",
        text
    );
}

#[rstest(
    expected_language,
    text,
    case(Arabic, "والموضوع"),
    case(Czech, "vývoj"),
    case(English, "massage"),
    case(English, "Hello"),
    // case(English, "super"),
    // case(English, "soup"),
    case(English, "I'm"),
    case(English, "Is"),
    case(English, "a"),
    // case(English, "I am"),
    // case(English, "I am a"),
)]
fn test_detect_top_one_reordered(expected_language: ScriptLanguage, text: &str) {
    let detector = DetectorBuilder::new(&MODELS_ALL_LANGUAGES_PRELOADED).build();
    assert_eq!(
        detector.detect_top_one_reordered(text),
        Some(expected_language),
        "{}",
        text
    );
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
fn test_detect_top_one_raw_is_deterministic(text: &str, languages: AHashSet<ScriptLanguage>) {
    let detector = DetectorBuilder::new(&MODELS_ALL_LANGUAGES_PRELOADED)
        .languages(languages.clone().into())
        .build();

    let mut detected_languages = AHashSet::new();
    for _ in 0..100 {
        let language = detector.detect_top_one_raw(text);
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
fn test_detect_top_one_raw_with_languages(
    expected_language: ScriptLanguage,
    text: &str,
    languages: AHashSet<ScriptLanguage>,
) {
    let detector = Detector::new(
        DetectorConfig::with_languages(languages.into()),
        &MODELS_ALL_LANGUAGES_PRELOADED,
    );
    let language = detector.detect_top_one_raw(text).unwrap();
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
fn test_no_text(invalid_str: &str) {
    let detector = DetectorBuilder::new(&MODELS_ALL_LANGUAGES_PRELOADED).build();
    assert_eq!(detector.detect_top_one_raw(invalid_str), None);
}

#[test]
fn test_max_trigrams_mode() {
    let detector = DetectorBuilder::new(&MODELS_ALL_LANGUAGES_PRELOADED)
        .languages(ahashset!(English, German))
        .max_trigrams()
        .build();

    assert!(detector.detect_top_one_raw("bed").is_some());
    assert!(detector.detect_top_one_raw("be").is_some());
    assert!(detector.detect_top_one_raw("b").is_some());

    assert!(detector.detect_top_one_raw("").is_none());
}

// Multiple
/* #[test]
fn test_detect_multiple_for_empty_string() {
    let detector = DetectorBuilder::new(&MODELS_ALL_LANGUAGES_PRELOADED).build();
    assert!(detector.probabilities_words("").is_empty());
}

#[rstest(
    expected_language,
    sentence,
    case(English, "massage"),
    case(English, "Hello"),
    case(English, "super"),
    case(English, "soup"),
    case(English, "I'm"),
    case(English, "Is"),
    case(English, "a"),
    case(
        English,
        "I'm really not sure whether multi-language detection is a good idea."
    ),
    case(English, "I am frightened! 🙈"),
    case(Kazakh, "төзімділік спорты")
)]
fn test_detect_multiple_with_one_language(expected_language: ScriptLanguage, sentence: &str) {
    let detector = DetectorBuilder::new(&MODELS_ALL_LANGUAGES_PRELOADED)
        .languages(ahashset!(
            ChineseMandarinSimplified,
            English,
            French,
            German,
            Kazakh,
            Russian,
            Spanish,
        ))
        .build();
    let words = detector.probabilities_words(sentence);

    for word in words {
        assert_eq!(
            word.probabilities.first().unwrap().0,
            expected_language,
            "{:?}",
            word.buf
        );
    }
} */
/*
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
