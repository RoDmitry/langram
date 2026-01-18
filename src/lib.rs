//! # Natural language detection library
//!
//! ## 346 ScriptLanguages (188 models + 158 scripts with no models)
//!
//! One language can be written in multiple scripts, so it will be detected as a different [`ScriptLanguage`](enum.ScriptLanguage.html) (language + script).
//!
//! `ISO 639-3` (using [`Language`](enum.Language.html#implementations)) and `ISO 15924` (using [`Script`](enum.Script.html#implementations))
//! are implemented, also combined using [`ScriptLanguage`](enum.ScriptLanguage.html#implementations).
//!
//! # Setup
//!
//! To use this library, you need a binary models file, which must be placed near the executable, or set `LANGRAM_MODELS_PATH`.
//!
//! It can be:
//!
//! * Downloaded from [langram_models releases](https://github.com/RoDmitry/langram_models/releases);
//!
//! * Built (recommened if big-endian target) [langram_models](https://github.com/RoDmitry/langram_models). Which is more advanced and allows you to remove model ngrams, and recompile, so that models binary would be lighter.
//!
//! # Example
//! ```rust
//! use langram::{DetectorBuilder, ModelsStorage};
//!
//! let models_storage = ModelsStorage::new().unwrap();
//! let detector = DetectorBuilder::new(&models_storage).build();
//!
//! // single thread
//! let text = "text";
//! let result = detector.detect_top_one_reordered(text);
//!
//! // or multithreaded (rayon for example)
//! use rayon::iter::IntoParallelRefIterator;
//! use rayon::iter::ParallelIterator;
//!
//! let texts = &["text1", "text2"];
//! let results: Vec<_> = texts
//!     .par_iter()
//!     .map(|text| detector.detect_top_one_reordered(text))
//!     .collect();
//! ```
//! `detector` also has [other methods](struct.Detector.html#implementations)

#[cfg(test)]
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
            _map
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
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
            _set
        }
    };
}

pub use alphabet_detector::{
    EnumCount, IntoEnumIterator, Language, Script, ScriptLanguage, UcdScript,
};

pub mod bin_storage;
mod detector;
pub mod model;
pub mod ngram_size;
mod ngrams;

pub use detector::{Detector, DetectorBuilder, ModelsStorage, ModelsStorageError};
pub use ngram_size::NgramSize;
