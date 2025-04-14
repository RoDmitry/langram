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

pub use alphabet_detector::ScriptLanguage;

pub const NGRAM_MAX_SIZE: usize = 5;

mod detector;
mod file_model;
mod fraction;
mod ngrams;

pub use detector::{Detector, DetectorConfig, ModelsStorage};
pub use file_model::FileModel;
pub use fraction::Fraction;
