use crate::{
    detector::{ModelNgrams, NgramFromChars},
    fraction::Fraction,
    NgramSize,
};
use ::std::{
    hash::Hash,
    io::{self, Cursor, ErrorKind, Read},
    iter::Map,
    path::PathBuf,
    str::{Chars, Split},
};
use alphabet_detector::ScriptLanguage;
use brotli::Decompressor;
use itertools::{IntoChunks, Itertools};
use serde_map::SerdeMap;

pub type FileModel = (usize, SerdeMap<Fraction, String>);

fn get_model(file_path: PathBuf) -> io::Result<FileModel> {
    // const, so it must be optimized out by the compiler
    if langram_models::MODELS_DIR.entries().len() < 2 {
        panic!("Models dir is empty. Path to `langram_models` crate must be changed.");
    }

    let compressed_file = langram_models::MODELS_DIR
        .get_file(file_path)
        .ok_or(ErrorKind::NotFound)?;
    let compressed_file_reader = Cursor::new(compressed_file.contents());
    let mut uncompressed_file = Decompressor::new(compressed_file_reader, 4096);
    let mut uncompressed_file_content = String::new();
    uncompressed_file.read_to_string(&mut uncompressed_file_content)?;

    // todo: from_reader once implemented
    serde_encom::from_str(&uncompressed_file_content).map_err(|e| e.into())
}

fn unwrap_model(model: io::Result<FileModel>, file_path: PathBuf) -> Option<FileModel> {
    match model {
        Ok(m) => Some(m),
        Err(e) => {
            if e.kind() != ErrorKind::NotFound {
                if cfg!(debug_assertions) {
                    panic!("Invalid model {file_path:?}: {e}: {e:?}");
                } else {
                    tracing::error!("Invalid model {file_path:?}: {e}: {e:?}");
                }
            }
            None
        }
    }
}

pub(crate) fn load_model(language: ScriptLanguage, ngram_size: NgramSize) -> Option<FileModel> {
    let file_path = PathBuf::from(language.into_str()).join(ngram_size.into_file_name());
    let model = get_model(file_path.clone());
    unwrap_model(model, file_path)
}

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
    file_model: Option<FileModel>,
    ngram_size: NgramSize,
) -> ModelNgrams<Ngram> {
    match file_model {
        Some(m) => {
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
        None => ModelNgrams::<Ngram>::with_capacity_and_hasher(1, Default::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngram_size::NgramSize;

    #[test]
    fn test_load_model() {
        load_model(ScriptLanguage::English, NgramSize::Uni).unwrap();
    }
}
