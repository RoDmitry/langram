use crate::{detector::ModelNgrams, fraction::Fraction, ngrams::NgramString};
use ::std::{
    io::{Cursor, ErrorKind, Read},
    path::PathBuf,
};
use ahash::AHashMap;
use alphabet_detector::ScriptLanguage;
use brotli::Decompressor;
use itertools::Itertools;
use serde_map::SerdeMap;

pub(crate) fn file_name_by_length(ngram_length: usize) -> &'static str {
    match ngram_length {
        1 => "unigrams.encom.br",
        2 => "bigrams.encom.br",
        3 => "trigrams.encom.br",
        4 => "quadrigrams.encom.br",
        5 => "fivegrams.encom.br",
        _ => panic!("ngram length {ngram_length} is not in range 1..6"),
    }
}

pub type FileModel = SerdeMap<Fraction, String>;

pub(crate) fn parse_model(file_model: FileModel, ngram_length: usize) -> ModelNgrams {
    let mut res = AHashMap::new();
    for (fraction, ngrams) in file_model {
        let floating_point_value = fraction.to_f64().ln();
        for ngram in &ngrams.chars().chunks(ngram_length) {
            res.insert(
                NgramString::try_from_chars(ngram).unwrap(),
                floating_point_value,
            );
        }
    }
    res
}

pub(crate) fn load_model(
    language: ScriptLanguage,
    ngram_length: usize,
) -> std::io::Result<FileModel> {
    if langram_models::MODELS_DIR.entries().len() < 2 {
        panic!("Models dir is empty. Path to `langram_models` crate must be changed.");
    }

    let file_name = file_name_by_length(ngram_length);
    let file_path = PathBuf::from(language.into_str()).join(file_name);
    let compressed_file = langram_models::MODELS_DIR
        .get_file(file_path)
        .ok_or(ErrorKind::NotFound)?;
    let compressed_file_reader = Cursor::new(compressed_file.contents());
    let mut uncompressed_file = Decompressor::new(compressed_file_reader, 4096);
    let mut uncompressed_file_content = String::new();
    uncompressed_file.read_to_string(&mut uncompressed_file_content)?;

    Ok(serde_encom::from_str(&uncompressed_file_content).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_model() {
        load_model(ScriptLanguage::English, 1).unwrap();
    }
}
