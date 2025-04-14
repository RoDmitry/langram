use ::std::collections::HashSet;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use langram::{Detector, DetectorConfig, ModelsStorage, ScriptLanguage};

// This is the common subset of languages that is supported by all
// language detection libraries in this benchmark.
const COMMON_LANGUAGES: &[ScriptLanguage] = &[
    ScriptLanguage::Arabic,
    // ScriptLanguage::ChineseSimplified,
    ScriptLanguage::Dutch,
    ScriptLanguage::English,
    ScriptLanguage::French,
    ScriptLanguage::German,
    ScriptLanguage::Hindi,
    ScriptLanguage::Italian,
    // ScriptLanguage::Japanese,
    // ScriptLanguage::Korean,
    ScriptLanguage::Portuguese,
    ScriptLanguage::Russian,
    ScriptLanguage::Spanish,
    ScriptLanguage::Swedish,
    ScriptLanguage::Turkish,
    // ScriptLanguage::Vietnamese,
];

const SENTENCES: &[&str] = &[
    "و في نفس الوقت أقول بأن الشيخ صالح لم يشر إلى مسؤولية الدولة التي تسمح لمواطنيها بملكية قنوات تبث ما تبث بل إنه حصر المسؤولية على ملاك هذه القنوات.",
    // "102年度彰化县劳工运动会暨园游会于12月1日(星期日)上午在县立体育场盛大登场，来自全县共61个事业单位及职业工会超过3,000多位选手参加，运动会场将展开一系列的竞技对战。",
    "Aan de fysieke gesteldheid van de aspirant-beoefenaar worden geen bijzondere eisen gesteld anders dan een goede gezondheid.",
    "Here, in a region abundant with natural beauty, golfers will surely be rewarded with an exceptional golf experience.",
    "Les affranchissements étaient très rares et s'ils accordaient la liberté à l'ancien esclave, ils ne lui conféraient pas le titre de citoyen.",
    "Natürlich war sie kein Pferd, dachte sie, aber warum wurde sie dann geritten, hatte einen Reiter zu tragen, war gesattelt, bekam Sporen und Lederpeitsche?",
    "अब इन्हें एक अलग प्लेट में निकाल कर गरमा-गरम आलू की सब्जी, हरे धनिये की चटनी या मीठी चटनी के साथ परोस कर खाइये और सबको खिलाइये।",
    "Alla fine del secolo cambiarono nome, divenendo uno Capitano e l’altro Difensore, ma mantenendo le stesse caratteristiche degli anni precedenti.",
    // "・京都大学施設に電離圏における電子数などの状況を取得可能なイオノゾンデ受信機（斜入射観測装置）を設置することで、新たな観測手法が地震先行現象検出に資するかを検証する。",
    // "아울러 가장 많은 수가 일하고 있는 직업은 곡식작물 재배자(109만6천명)로 조사됐고, 상점판매 및 관리인(97만8천명), 상점판매원(87만3천명), 일반 영업원(59만명) 등이 뒤를 이었다.",
    "Dizer que não estou, significaria explicar porquê e não me apetece nada desfiar o rosário das minhas lamentações.",
    "То есть присяжные не сочли возможным осудить за соучастие в убийстве и убийство людей, доказательства вины которых не были предъявлены.",
    "Con frecuencia creo que Francia es malinterpretada, seala, aludiendo a la imagen que tiene el pas internacionalmente en materia de tica de trabajo.",
    "Med dagens stadshusmajoritet är det övervikt för ett enplanstorg med bostäder, alltså för en ombyggnad i linje med alternativ maxi.",
    "Mezuniyet hediyesi olarak yerleşkenin kuzey batı bölümüne dikilmiş vişnelerin meyvesini, tohumunu almışlardır.",
    // "Cuốn sách là cẩm nang hữu ích để tham khảo và học hỏi, giúp các bà mẹ Việt tự tin hơn trong cách dạy con.",
];

fn benchmark_detector(c: &mut Criterion) {
    let mut group_all_preloaded = c.benchmark_group("Detector all languages preloaded");

    let models_storage = ModelsStorage::default();
    let config_all_ngrams_all_languages = DetectorConfig::new_all_languages();
    let detector_all_ngrams_all_languages =
        Detector::new(config_all_ngrams_all_languages, &models_storage);
    detector_all_ngrams_all_languages.preload_models();

    group_all_preloaded.bench_function("all ngrams", |bencher| {
        bencher.iter(|| {
            SENTENCES.iter().for_each(|sentence| {
                black_box(detector_all_ngrams_all_languages.detect_top_one(sentence, 0.0));
            });
        });
    });

    let config_max_trigrams_all_languages = DetectorConfig::new_all_languages().max_trigrams();
    let detector_max_trigrams_all_languages =
        Detector::new(config_max_trigrams_all_languages, &models_storage);
    group_all_preloaded.bench_function("max trigrams", |bencher| {
        bencher.iter(|| {
            SENTENCES.iter().for_each(|sentence| {
                black_box(detector_max_trigrams_all_languages.detect_top_one(sentence, 0.0));
            });
        });
    });
    group_all_preloaded.finish();

    /* let mut group2 = c.benchmark_group("Detector with all languages in multiple threads");
    group2.bench_function("max trigrams", |bencher| {
        bencher.iter(|| {
            sentences.par_iter().for_each(|sentence| {
                black_box(max_trigrams_detector_all_languages_preloaded.detect(*sentence));
            });
        });
    });
    group2.bench_function("all ngrams", |bencher| {
        bencher.iter(|| {
            sentences.par_iter().for_each(|sentence| {
                black_box(all_ngrams_detector_all_languages_preloaded.detect(*sentence));
            });
        });
    });
    group2.finish(); */

    let mut group_common_preloaded = c.benchmark_group("Detector common languages");

    let config_all_ngrams_common_languages = DetectorConfig::with_languages(
        COMMON_LANGUAGES
            .iter()
            .copied()
            .collect::<HashSet<_, ahash::RandomState>>(),
    );
    let detector_all_ngrams_common_languages =
        Detector::new(config_all_ngrams_common_languages, &models_storage);
    group_common_preloaded.bench_function("all ngrams", |bencher| {
        bencher.iter(|| {
            SENTENCES.iter().for_each(|sentence| {
                black_box(detector_all_ngrams_common_languages.detect_top_one(sentence, 0.0));
            });
        });
    });

    let config_max_trigrams_common_languages = DetectorConfig::with_languages(
        COMMON_LANGUAGES
            .iter()
            .copied()
            .collect::<HashSet<_, ahash::RandomState>>(),
    )
    .max_trigrams();
    let detector_max_trigrams_common_languages =
        Detector::new(config_max_trigrams_common_languages, &models_storage);
    group_common_preloaded.bench_function("max trigrams", |bencher| {
        bencher.iter(|| {
            SENTENCES.iter().for_each(|sentence| {
                black_box(detector_max_trigrams_common_languages.detect_top_one(sentence, 0.0));
            });
        });
    });
    group_common_preloaded.finish();

    /* let mut group4 = c.benchmark_group("Detector with common languages in multiple threads");
    group4.bench_function("max trigrams mode", |bencher| {
        bencher.iter(|| {
            sentences.par_iter().for_each(|sentence| {
                black_box(max_trigrams_detector_common_languages.detect(*sentence));
            });
        });
    });
    group4.bench_function("all ngrams mode", |bencher| {
        bencher.iter(|| {
            sentences.par_iter().for_each(|sentence| {
                black_box(
                    all_ngrams_detector_common_languages.detect(*sentence),
                );
            });
        });
    });
    group4.finish(); */
}

fn benchmark_preload_all_languages(c: &mut Criterion) {
    let mut group = c.benchmark_group("Detector preload");
    group.sample_size(10);
    group.bench_function("all languages", |bencher| {
        bencher.iter(|| {
            let models_storage = ModelsStorage::default();
            let config_all_ngrams_all_languages = DetectorConfig::new_all_languages();
            let detector = Detector::new(config_all_ngrams_all_languages, &models_storage);
            detector.preload_models();
        })
    });
}

criterion_group!(benches, benchmark_detector, benchmark_preload_all_languages,);

criterion_main!(benches);
