use crate::model::TrainingModel;
use ::std::{
    fs::{create_dir_all, File},
    io,
    io::Write,
    path::Path,
};
use ahash::AHashMap;
use alphabet_detector::{filter_max, Script, ScriptLanguage};
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
    let is_han = ScriptLanguage::all_with_script(Script::Han).contains(&language);
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
            chars.retain(|&ch| Script::find(ch) == Script::Han);
            !chars.is_empty()
        });
    }

    let unigram_model = TrainingModel::from_text(&word_chars, 1, AHashMap::new());
    unigram_model
        .to_file_model()
        .write_compressed(&out_mod_path.join("unigrams.encom.br"))?;
    let TrainingModel {
        absolute_frequencies,
        ..
    } = unigram_model;

    let bigram_model = TrainingModel::from_text(&word_chars, 2, absolute_frequencies);
    bigram_model
        .to_file_model()
        .write_compressed(&out_mod_path.join("bigrams.encom.br"))?;
    if is_han {
        return Ok(());
    }
    let TrainingModel {
        absolute_frequencies,
        ..
    } = bigram_model;

    let trigram_model = TrainingModel::from_text(&word_chars, 3, absolute_frequencies);
    trigram_model
        .to_file_model()
        .write_compressed(&out_mod_path.join("trigrams.encom.br"))?;
    let TrainingModel {
        absolute_frequencies,
        ..
    } = trigram_model;

    let quadrigram_model = TrainingModel::from_text(&word_chars, 4, absolute_frequencies);
    quadrigram_model
        .to_file_model()
        .write_compressed(&out_mod_path.join("quadrigrams.encom.br"))?;
    let TrainingModel {
        absolute_frequencies,
        ..
    } = quadrigram_model;

    let fivegram_model = TrainingModel::from_text(&word_chars, 5, absolute_frequencies);
    fivegram_model
        .to_file_model()
        .write_compressed(&out_mod_path.join("fivegrams.encom.br"))
}
