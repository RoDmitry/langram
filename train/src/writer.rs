use crate::model::TrainingModel;
use ::std::{
    fs::{create_dir_all, File},
    io,
    io::Write,
    path::Path,
};
use ahash::AHashMap;
use alphabet_detector::{filter_max, ScriptLanguage, UcdScript};
use brotli::CompressorWriter;
use itertools::Itertools;
use langram::FileModel;

pub trait FileModelWriter {
    fn write_compressed(&self, file_path: &Path) -> io::Result<()>;
}

impl FileModelWriter for FileModel {
    fn write_compressed(&self, file_path: &Path) -> io::Result<()> {
        if let Some(parent) = file_path.parent() {
            create_dir_all(parent)?;
        }
        let file = File::create(file_path)?;
        let mut compressed_file = CompressorWriter::new(file, 4096, 11, 22);
        let ser = serde_encom::to_string(&self).unwrap();
        compressed_file.write_all(ser.as_bytes())
    }
}

/// Creates language model files and writes them to a directory.
pub fn create_model_and_write_files(
    out_mod_path: &Path,
    char_indices: impl Iterator<Item = (usize, char)>,
    language: ScriptLanguage,
) -> io::Result<()> {
    let words = alphabet_detector::words::from_ch_ind(char_indices);
    let is_han = UcdScript::from(language) == UcdScript::Han;
    let mut word_chars: Vec<Vec<char>> = words
        // .inspect(|wld| println!("{:?}", wld))
        // filter
        .filter_map(|wld| {
            // no filter for `Script::Han`
            if is_han {
                return Some(wld.buf);
            }

            if filter_max(wld.langs_cnt).0.contains(&language) {
                Some(wld.buf)
            } else {
                None
            }
        })
        // .map(|wld| wld.buf)
        .collect();

    if is_han {
        word_chars.retain_mut(|chars| {
            chars.retain(|&ch| UcdScript::find(ch) == UcdScript::Han);
            !chars.is_empty()
        });
    }

    let unigram_model = TrainingModel::new_windows(&word_chars, AHashMap::new(), 1);
    unigram_model
        .to_file_model(&[])
        .write_compressed(&out_mod_path.join("unigrams.encom.br"))?;
    let TrainingModel {
        absolute_frequencies,
        ..
    } = unigram_model;

    println!("{:?} processing bigrams", out_mod_path.file_name().unwrap());
    let bigram_model = TrainingModel::new_windows(&word_chars, absolute_frequencies, 2);
    bigram_model
        .to_file_model(&[])
        .write_compressed(&out_mod_path.join("bigrams.encom.br"))?;
    if is_han {
        return Ok(());
    }
    let TrainingModel {
        absolute_frequencies,
        ..
    } = bigram_model;

    println!(
        "{:?} processing trigrams",
        out_mod_path.file_name().unwrap()
    );
    let trigram_model = TrainingModel::new_windows(&word_chars, absolute_frequencies, 3);
    trigram_model
        .to_file_model(&[])
        .write_compressed(&out_mod_path.join("trigrams.encom.br"))?;
    let TrainingModel {
        absolute_frequencies,
        ..
    } = trigram_model;

    println!(
        "{:?} processing quadrigrams",
        out_mod_path.file_name().unwrap()
    );
    let quadrigram_model = TrainingModel::new_windows(&word_chars, absolute_frequencies, 4);
    quadrigram_model
        .to_file_model(&[])
        .write_compressed(&out_mod_path.join("quadrigrams.encom.br"))?;
    let TrainingModel {
        absolute_frequencies,
        ..
    } = quadrigram_model;

    println!(
        "{:?} processing fivegrams",
        out_mod_path.file_name().unwrap()
    );
    let fivegram_model = TrainingModel::new_windows(&word_chars, absolute_frequencies, 5);
    fivegram_model
        .to_file_model(&[])
        .write_compressed(&out_mod_path.join("fivegrams.encom.br"))?;
    drop(fivegram_model);

    println!(
        "{:?} processing wordgrams",
        out_mod_path.file_name().unwrap()
    );
    let wordgram_model = TrainingModel::new(&word_chars, AHashMap::new());
    wordgram_model
        .to_file_model(&[' '])
        .write_compressed(&out_mod_path.join("wordgrams.encom.br"))
}
