use crate::{
    detector::{ModelNgrams, NgramsSize},
    fraction::Fraction,
    ngrams::NgramString,
};
use ::std::{
    io,
    io::{Cursor, ErrorKind, Read},
    path::PathBuf,
};
use alphabet_detector::ScriptLanguage;
use brotli::Decompressor;
use itertools::Itertools;
use serde_map::SerdeMap;

pub type FileModel = (usize, SerdeMap<Fraction, String>);

pub(crate) fn parse_model(
    file_model: io::Result<FileModel>,
    ngram_size: NgramsSize,
) -> ModelNgrams {
    match file_model {
        Ok(m) => {
            let mut res = ModelNgrams::with_capacity_and_hasher(m.0, Default::default());
            for (fraction, ngrams) in m.1 {
                let floating_point_value = fraction.to_f64().ln();
                for ngram in &ngrams.chars().chunks(ngram_size as usize + 1) {
                    res.insert(
                        NgramString::try_from_chars(ngram).unwrap(),
                        floating_point_value,
                    );
                }
            }
            res.shrink_to_fit();
            res
        }
        _ => ModelNgrams::with_capacity_and_hasher(1, Default::default()),
    }
}

pub(crate) fn load_model(language: ScriptLanguage, file_name: &str) -> std::io::Result<FileModel> {
    if langram_models::MODELS_DIR.entries().len() < 2 {
        panic!("Models dir is empty. Path to `langram_models` crate must be changed.");
    }

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
    use crate::detector::NgramsSize;

    #[test]
    fn test_load_model() {
        load_model(ScriptLanguage::English, NgramsSize::Uni.into_file_name()).unwrap();
    }
}
