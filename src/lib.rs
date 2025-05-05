//! # Natural language detection library
//!
//! ## 308 ScriptLanguages (187 models + 121 single language scripts)
//!
//! One language can be written in multiple scripts, so it will be detected as a different [`ScriptLanguage`](enum.ScriptLanguage.html) (language + script).
//!
//! `ISO 639-3` (using [`Language`](enum.Language.html#implementations)) and `ISO 15924` (using [`Script`](enum.Script.html#implementations))
//! are implemented, also combined using [`ScriptLanguage`](enum.ScriptLanguage.html#implementations).
//!
//! # Example
//! ```
//! use ::std::sync::LazyLock;
//! use langram::*;
//! use rayon::iter::IntoParallelRefIterator;
//! use rayon::iter::ParallelIterator;
//!
//! static LANGRAM_MODELS: LazyLock<ModelsStorage> = LazyLock::new(|| {
//!     ModelsStorage::preloaded::<ahash::RandomState>(ScriptLanguage::all().collect())
//! });
//! let config = DetectorConfig::new_all_languages();
//! let detector = Detector::new(config, &LANGRAM_MODELS);
//! let texts = &["text1", "text2"];
//! // multithreaded iter
//! let result: Vec<_> = texts
//!     .par_iter()
//!     .map(|text| detector.detect_top_one(text, 0.0))
//!     .collect();
//! ```
//! But `detector` has [other methods](struct.Detector.html#implementations)

#[allow(unused_macros)]
macro_rules! ahashmap {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(ahashmap!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { ahashmap!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = ahashmap!(@count $($key),*);
            let mut _map = ::ahash::AHashMap::with_capacity(_cap);
            $(
                let _ = _map.insert($key, $value);
            )*
            _map.into()
        }
    };
}

#[allow(unused_macros)]
macro_rules! ahashset {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(ahashset!(@single $rest)),*]));

    ($($key:expr,)+) => { ahashset!($($key),+) };
    ($($key:expr),*) => {
        {
            let _cap = ahashset!(@count $($key),*);
            let mut _set = ::ahash::AHashSet::with_capacity(_cap);
            $(
                let _ = _set.insert($key);
            )*
            _set.into()
        }
    };
}

pub use alphabet_detector::{Language, Script, ScriptLanguage, UcdScript};

mod detector;
mod file_model;
mod fraction;
mod ngrams;

pub use detector::{Detector, DetectorConfig, ModelsStorage, NgramSize};
pub use file_model::FileModel;
pub use fraction::Fraction;
