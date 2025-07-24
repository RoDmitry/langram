//! # Natural language detection library
//!
//! ## 314 ScriptLanguages (187 models + 127 single language scripts)
//!
//! One language can be written in multiple scripts, so it will be detected as a different [`ScriptLanguage`](enum.ScriptLanguage.html) (language + script).
//!
//! `ISO 639-3` (using [`Language`](enum.Language.html#implementations)) and `ISO 15924` (using [`Script`](enum.Script.html#implementations))
//! are implemented, also combined using [`ScriptLanguage`](enum.ScriptLanguage.html#implementations).
//!
//! # Example
//! ```rust
//! use langram::{DetectorBuilder, ModelsStorage};
//!
//! let models_storage = ModelsStorage::default();
//! let detector = DetectorBuilder::new(&models_storage).build();
//! // preload models for faster detection
//! detector.preload_models();
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
mod ngram_size;
mod ngrams;

pub use detector::{Detector, DetectorBuilder, ModelsStorage};
pub use file_model::FileModel;
pub use fraction::Fraction;
pub use ngram_size::NgramSize;
