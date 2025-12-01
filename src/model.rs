use crate::ngram_size::NgramSize;
use ::std::collections::HashMap;
use strum::EnumCount;

// pub type ModelNgrams = entropy_map::Map<String, f64, 64, 10, u16, rustc_hash::FxHasher>;
pub type ModelNgrams = HashMap<String, f64, rustc_hash::FxBuildHasher>;
pub type Model = [ModelNgrams; NgramSize::COUNT];
