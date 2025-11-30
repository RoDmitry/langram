use crate::fraction::Fraction;
use ::std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};
use brotli::Decompressor;
use debug_unsafe::slice::SliceGetter;
use itertools::Itertools;
use langram::{
    model::{Model, ModelNgrams},
    NgramSize,
};
use serde_map::SerdeMap;
use strum::IntoEnumIterator;
use thiserror::Error;
/* use std::{
    iter::Map,
    str::{Chars, Split},
}; */

pub type FileModel = (usize, SerdeMap<Fraction, String>);

/* pub(crate) trait IntoIteratorBorrowed<'i>: Sized + Clone {
    fn to_iter(&self) -> impl Iterator<Item = String>;
}

impl<'i> IntoIteratorBorrowed<'i> for IntoChunks<Chars<'i>> {
    #[inline(always)]
    fn to_iter(&self) -> impl Iterator<Item = String> {
        self.into_iter().map(|c| c.collect::<String>())
    }
}

impl<'i, T> IntoIteratorBorrowed<'i> for Map<Split<'i, char>, T>
where
    T: FnMut(&'i str) -> String + Clone,
{
    #[inline(always)]
    fn to_iter(&self) -> impl Iterator<Item = String> {
        // todo: optimize somehow?
        self.to_owned()
    }
} */

pub(crate) struct SpaceNgramsUnpacker;
pub(crate) struct ChunksNgramsUnpacker;

/* pub(crate) trait NgramsUnpacker: Sized {
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
        ngrams.split(' ').map(|s| s.to_owned())
    }
} */

pub(crate) trait NgramsUnpacker: Sized {
    // unnecessary Vec
    fn unpack(ngrams: String, ngram_size: NgramSize) -> Vec<String>;
}

impl NgramsUnpacker for ChunksNgramsUnpacker {
    #[inline(always)]
    fn unpack(ngrams: String, ngram_size: NgramSize) -> Vec<String> {
        ngrams
            .chars()
            .chunks(ngram_size as usize + 1)
            .into_iter()
            .map(|s| s.collect())
            .collect()
    }
}

impl NgramsUnpacker for SpaceNgramsUnpacker {
    #[inline(always)]
    fn unpack(ngrams: String, _ngram_size: NgramSize) -> Vec<String> {
        ngrams.split(' ').map(|s| s.to_owned()).collect()
    }
}

#[derive(Error, Debug)]
pub enum ModelConversionError {
    #[error("Read error")]
    Read(#[source] io::Error),
    #[error("SerdeEncom error")]
    SerdeEncom(#[from] serde_encom::Error),
}

fn read(file: File) -> Result<FileModel, ModelConversionError> {
    let mut uncompressed_file = Decompressor::new(file, 4096);
    let mut uncompressed_file_content = String::new();
    uncompressed_file
        .read_to_string(&mut uncompressed_file_content)
        .map_err(ModelConversionError::Read)?;

    // todo: from_reader once implemented
    serde_encom::from_str(&uncompressed_file_content).map_err(|e| e.into())
}

fn parse_model<NU: NgramsUnpacker>(file_model: FileModel, ngram_size: NgramSize) -> ModelNgrams {
    let iter = file_model.1.into_iter().flat_map(|(fraction, ngrams)| {
        let floating_point_value = fraction.to_f64().ln();
        let unp = NU::unpack(ngrams, ngram_size);
        unp.into_iter()
            .map(move |chars| (chars, floating_point_value))
    });
    iter.collect()
}

pub fn dir_into_model(lang_dir: PathBuf) -> Result<Option<Model>, ModelConversionError> {
    if lang_dir.is_dir() {
        let mut res = Model::default();
        for ngram_size in NgramSize::iter() {
            let file_name = crate::into_file_name(ngram_size);
            if let Ok(file) = File::open(lang_dir.join(file_name)) {
                let file_model = read(file)?;
                let ngram_map = if ngram_size == NgramSize::Word {
                    parse_model::<SpaceNgramsUnpacker>(file_model, ngram_size)
                } else {
                    parse_model::<ChunksNgramsUnpacker>(file_model, ngram_size)
                };
                *res.ngrams.get_safe_unchecked_mut(ngram_size as usize) = ngram_map;
            }
        }

        let uni_model = res.ngrams.get_safe_unchecked(NgramSize::Uni as usize);
        if uni_model.is_empty() {
            return Ok(None);
        }
        res.ngram_min_probability = Model::compute_min_probability(uni_model.len());

        Ok(Some(res))
    } else {
        Ok(None)
    }
}
