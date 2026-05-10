use crate::bin_storage::{ArchivedBinStorage, StorageNgrams, StorageNgramsArr};
#[cfg(test)]
use crate::model::Model;
use ::std::{
    env, fmt,
    fs::{self, File},
    io::{self, copy, BufReader, BufWriter},
    path::PathBuf,
};
use alphabet_detector::{ScriptLanguage, ScriptLanguageArr};
use brotli_decompressor::Decompressor;
use memmap2::Mmap;
use reqwest::blocking::get;
use rkyv::Archive;
use thiserror::Error;

pub(super) type NgramModel = <StorageNgrams as Archive>::Archived;
type NgramModelArr = <StorageNgramsArr as Archive>::Archived;

pub struct ModelsStorage<'m> {
    #[allow(unused)]
    mmap: Mmap,
    pub(super) langs_ngram_min_probability: &'m <ScriptLanguageArr<f64> as Archive>::Archived,
    pub(super) ngrams: &'m NgramModelArr,
    pub(super) wordgrams: &'m NgramModel,
    pub(super) wordgram_min_probability: f64,
}

impl fmt::Debug for ModelsStorage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelsStorage")
            .field("wordgram_min_probability", &self.wordgram_min_probability)
            .finish_non_exhaustive()
    }
}

impl<'m> ModelsStorage<'m> {
    pub const FILE_NAME: &'static str = "langram_models.bin";

    #[inline]
    fn get_file() -> Result<File, ModelsStorageError> {
        let path = match env::var("LANGRAM_MODELS_PATH") {
            Ok(path_str) => {
                let mut path = PathBuf::from(path_str);
                if path.is_file() {
                    if let Ok(file) = File::open(&path) {
                        return Ok(file);
                    }
                    path.pop();
                }
                path
            }
            Err(_) => {
                let mut path_near = env::current_exe().map_err(ModelsStorageError::CurrentExe)?;
                path_near.pop();
                path_near
            }
        };

        let file_path = path.join(Self::FILE_NAME);
        if let Ok(file) = File::open(&file_path) {
            return Ok(file);
        }

        let part_file_path = path.join(concat_const::concat!(ModelsStorage::FILE_NAME, ".part"));
        if part_file_path.exists() {
            fs::remove_file(&part_file_path).map_err(ModelsStorageError::ModelsPartFileRemove)?;
        }

        let file =
            File::create_new(&part_file_path).map_err(ModelsStorageError::ModelsPartFileCreate)?;
        let mut writer = BufWriter::new(file);

        let compressed_file_path =
            path.join(concat_const::concat!(ModelsStorage::FILE_NAME, ".br"));
        if let Ok(compressed_file) = File::open(compressed_file_path) {
            let reader = BufReader::new(compressed_file);

            // Brotli decompressor
            let mut decompressor = Decompressor::new(reader, 4096);
            // Stream decompressed bytes into output file
            copy(&mut decompressor, &mut writer).map_err(ModelsStorageError::StreamDecompressor)?;
        } else {
            println!("Downloading langram models...");
            let response = get(
                "https://github.com/RoDmitry/langram_models/releases/download/v0.11/langram_models.bin.br"
            ).map_err(ModelsStorageError::Download)?;

            // Brotli decompressor
            let mut decompressor = Decompressor::new(response, 64 * 1024);
            // Stream decompressed bytes into output file
            copy(&mut decompressor, &mut writer).map_err(ModelsStorageError::StreamDecompressor)?;
            println!("Downloaded langram models");
        }

        // Also ensures all buffered data is written
        let file = writer
            .into_inner()
            .map_err(ModelsStorageError::WriterIntoInner)?;

        // Optional but safer against power loss
        file.sync_all().map_err(ModelsStorageError::FileSync)?;

        fs::rename(part_file_path, file_path).map_err(ModelsStorageError::ModelsPartFileRename)?;

        Ok(file)
    }

    #[inline]
    pub fn new() -> Result<Self, ModelsStorageError> {
        let file = Self::get_file()?;

        let mmap = unsafe { Mmap::map(&file) }.map_err(ModelsStorageError::Mmap)?;

        Self::from_mmap(mmap)
    }

    #[inline]
    fn from_mmap(mmap: Mmap) -> Result<Self, ModelsStorageError> {
        // SAFETY: slice has the same lifetime as mmap
        let slice: &'m [u8] = unsafe { ::core::mem::transmute::<&[u8], &'m [u8]>(mmap.as_ref()) };
        let fs = rkyv::access::<ArchivedBinStorage, rkyv::rancor::Error>(slice)?;

        if fs.hash != ScriptLanguage::HASH {
            return Err(ModelsStorageError::ModelsHash(fs.hash.to_native()));
        }

        Ok(Self {
            mmap,
            langs_ngram_min_probability: &fs.langs_ngram_min_probability,
            ngrams: &fs.ngrams,
            wordgrams: &fs.wordgrams,
            wordgram_min_probability: fs.wordgram_min_probability.to_native(),
        })
    }

    #[cfg(test)]
    pub fn from_models(input: impl IntoIterator<Item = (ScriptLanguage, Model)>) -> Self {
        let mut file_storage = crate::bin_storage::BinStorage::default();

        for (l, m) in input {
            file_storage.add(l, m);
        }
        file_storage.finalize();

        let bytes = file_storage.to_bytes().unwrap();
        let mmap = mmap_from_bytes(&bytes).unwrap();

        Self::from_mmap(mmap).unwrap()
    }
}

#[cfg(test)]
fn mmap_from_bytes(data: &[u8]) -> ::std::io::Result<Mmap> {
    let mut mmap = memmap2::MmapMut::map_anon(data.len())?;
    mmap.copy_from_slice(data);
    mmap.make_read_only()
}

#[derive(Error, Debug)]
pub enum ModelsStorageError {
    #[error("Current exe error")]
    CurrentExe(#[source] io::Error),
    #[error("Models part file create error")]
    ModelsPartFileCreate(#[source] io::Error),
    #[error("Models part file remove error")]
    ModelsPartFileRemove(#[source] io::Error),
    #[error("Models part file rename error")]
    ModelsPartFileRename(#[source] io::Error),
    #[error("Failed to stream decompressed model bytes into file")]
    StreamDecompressor(#[source] io::Error),
    #[error("Failed to download models")]
    Download(#[source] reqwest::Error),
    #[error("Failed to flush the buffer")]
    WriterIntoInner(#[source] io::IntoInnerError<BufWriter<File>>),
    #[error("Failed to sync file data to disk")]
    FileSync(#[source] io::Error),
    #[error("Mmap error")]
    Mmap(#[source] io::Error),
    #[error("Rkyv access error")]
    RkyvAccess(#[from] rkyv::rancor::Error),
    #[error("Langram models hash {0:X} is incompatible, please recompile models!")]
    ModelsHash(u64),
}
