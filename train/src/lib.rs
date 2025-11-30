use langram::NgramSize;

pub mod file_model;
mod fraction;
mod training_model;
mod writer;

pub use writer::create_model_and_write_files;

// not possible to have const fn in traits
#[inline]
pub const fn into_file_name(size: NgramSize) -> &'static str {
    use NgramSize::*;
    match size {
        Uni => "unigrams.encom.br",
        Bi => "bigrams.encom.br",
        Tri => "trigrams.encom.br",
        Quadri => "quadrigrams.encom.br",
        Five => "fivegrams.encom.br",
        Word => "wordgrams.encom.br",
    }
}
