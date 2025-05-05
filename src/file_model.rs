use crate::{
    detector::{ModelNgrams, NgramFromChars, NgramSize},
    fraction::Fraction,
};
use ::std::{
    hash::Hash,
    io,
    io::{Cursor, ErrorKind, Read},
    iter::Map,
    path::PathBuf,
    str::{Chars, Split},
};
use alphabet_detector::ScriptLanguage;
use brotli::Decompressor;
use itertools::{IntoChunks, Itertools};
use serde_map::SerdeMap;

pub type FileModel = (usize, SerdeMap<Fraction, String>);

pub(crate) trait IntoIteratorBorrowed<'i>: Sized + Clone {
    fn to_iter(&self) -> impl Iterator<Item = impl Iterator<Item = char>>;
}

impl<'i> IntoIteratorBorrowed<'i> for IntoChunks<Chars<'i>> {
    #[inline(always)]
    fn to_iter(&self) -> impl Iterator<Item = impl Iterator<Item = char>> {
        self.into_iter()
    }
}

impl<'i, T: FnMut(&'i str) -> Chars<'i> + Clone> IntoIteratorBorrowed<'i>
    for Map<Split<'i, char>, T>
{
    #[inline(always)]
    fn to_iter(&self) -> impl Iterator<Item = impl Iterator<Item = char>> {
        // todo: optimize somehow?
        self.to_owned()
    }
}

pub(crate) struct ChunksNgramsUnpacker;
pub(crate) struct SpaceNgramsUnpacker;

pub(crate) trait NgramsUnpacker: Sized {
    fn unpack<'a>(ngrams: &'a str, ngram_size: NgramSize) -> impl IntoIteratorBorrowed<'a>;
}

impl NgramsUnpacker for ChunksNgramsUnpacker {
    #[inline(always)]
    fn unpack<'a>(ngrams: &'a str, ngram_size: NgramSize) -> impl IntoIteratorBorrowed<'a> {
        ngrams.chars().chunks(ngram_size as usize + 1)
    }
}

impl NgramsUnpacker for SpaceNgramsUnpacker {
    #[inline(always)]
    fn unpack<'a>(ngrams: &'a str, _ngram_size: NgramSize) -> impl IntoIteratorBorrowed<'a> {
        ngrams.split(' ').map(|s| s.chars())
    }
}

pub(crate) fn parse_model<Ngram: NgramFromChars + Eq + Hash, NU: NgramsUnpacker>(
    file_model: io::Result<FileModel>,
    ngram_size: NgramSize,
) -> ModelNgrams<Ngram> {
    match file_model {
        Ok(m) => {
            // somehow extra initial space, makes detection faster, and gives more stable benchmark results
            let mut res =
                ModelNgrams::<Ngram>::with_capacity_and_hasher(m.0 << 1, Default::default());
            for (fraction, ngrams) in m.1 {
                let floating_point_value = fraction.to_f64().ln();
                for ngram in NU::unpack(&ngrams, ngram_size).to_iter() {
                    res.insert(Ngram::from_chars(ngram), floating_point_value);
                }
            }
            res.shrink_to_fit();
            res
        }
        _ => ModelNgrams::<Ngram>::with_capacity_and_hasher(1, Default::default()),
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
    use crate::detector::NgramSize;

    #[test]
    fn test_load_model() {
        load_model(ScriptLanguage::English, NgramSize::Uni.into_file_name()).unwrap();
    }
}
