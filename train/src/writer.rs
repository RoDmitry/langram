use crate::{file_model::FileModel, training_model::TrainingModel};
use ::std::{
    fs::{create_dir_all, File},
    io,
    io::Write,
    path::Path,
};
use ahash::AHashMap;
use alphabet_detector::{ScriptLanguage, UcdScript};
use brotli::CompressorWriter;
use debug_unsafe::slice::SliceGetter;
use langram::NgramSize;

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
        .filter_map(|wld: alphabet_detector::Word<Vec<char>>| {
            // no filter for `Script::Han`
            if is_han {
                return Some(wld.buf);
            }

            if *wld.langs_cnt.get_safe_unchecked(language as usize) == wld.buf.len() as u32 {
                Some(wld.buf)
            } else {
                None
            }
        })
        .collect();

    if is_han {
        word_chars.retain_mut(|chars| {
            chars.retain(|&ch| UcdScript::find(ch) == UcdScript::Han);
            !chars.is_empty()
        });
    }

    println!(
        "{:?} processing unigrams",
        out_mod_path.file_name().unwrap()
    );
    let unigram_model = TrainingModel::new_windows(&word_chars, 1);
    unigram_model
        .to_file_model(AHashMap::new(), &[])
        .write_compressed(&out_mod_path.join(crate::into_file_name(NgramSize::Uni)))?;
    let TrainingModel {
        absolute_frequencies,
    } = unigram_model;

    // adds underscores '_'
    // have been tested, it makes detection worse.
    /* let word_chars_with_underscores: Vec<_> = word_chars
    .iter()
    .map(|w| {
        let mut wnew = Vec::with_capacity(w.len() + 2);
        wnew.push('_');
        wnew.extend(w.iter());
        wnew.push('_');
        wnew
    })
    .collect(); */

    println!("{:?} processing bigrams", out_mod_path.file_name().unwrap());
    let bigram_model = TrainingModel::new_windows(&word_chars, 2);
    bigram_model
        .to_file_model(absolute_frequencies, &[])
        .write_compressed(&out_mod_path.join(crate::into_file_name(NgramSize::Bi)))?;

    if is_han {
        return Ok(());
    }
    let TrainingModel {
        absolute_frequencies,
    } = bigram_model;

    println!(
        "{:?} processing trigrams",
        out_mod_path.file_name().unwrap()
    );
    let trigram_model = TrainingModel::new_windows(&word_chars, 3);
    trigram_model
        .to_file_model(absolute_frequencies, &[])
        .write_compressed(&out_mod_path.join(crate::into_file_name(NgramSize::Tri)))?;
    let TrainingModel {
        absolute_frequencies,
    } = trigram_model;

    println!(
        "{:?} processing quadrigrams",
        out_mod_path.file_name().unwrap()
    );
    let quadrigram_model = TrainingModel::new_windows(&word_chars, 4);
    quadrigram_model
        .to_file_model(absolute_frequencies, &[])
        .write_compressed(&out_mod_path.join(crate::into_file_name(NgramSize::Quadri)))?;
    let TrainingModel {
        absolute_frequencies,
    } = quadrigram_model;

    println!(
        "{:?} processing fivegrams",
        out_mod_path.file_name().unwrap()
    );
    let fivegram_model = TrainingModel::new_windows(&word_chars, 5);
    fivegram_model
        .to_file_model(absolute_frequencies, &[])
        .write_compressed(&out_mod_path.join(crate::into_file_name(NgramSize::Five)))?;
    drop(fivegram_model);

    println!(
        "{:?} processing wordgrams",
        out_mod_path.file_name().unwrap()
    );
    let wordgram_model = TrainingModel::new(&word_chars);
    wordgram_model
        .to_file_model(AHashMap::new(), &[' '])
        .write_compressed(&out_mod_path.join(crate::into_file_name(NgramSize::Word)))
}
