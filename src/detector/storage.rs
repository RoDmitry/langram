use crate::bin_storage::{ArchivedBinStorage, BinStorage, StorageNgrams, StorageNgramsArr};
#[cfg(test)]
use crate::model::Model;
use ::std::{fmt, io};
use alphabet_detector::{ScriptLanguage, ScriptLanguageArr};
use memmap2::Mmap;
use rkyv::Archive;
use std::{env, fs::File, path::Path};
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
    #[inline]
    fn get_file() -> Result<File, ModelsStorageError> {
        if let Ok(path) = env::var("LANGRAM_MODELS_PATH") {
            let path = Path::new(&path);
            if let Ok(file) = File::open(path) {
                return Ok(file);
            }
        };

        let mut path = env::current_exe().map_err(ModelsStorageError::CurrentExe)?;
        path.pop();
        path.push(BinStorage::FILE_NAME);
        if let Ok(file) = File::open(path) {
            return Ok(file);
        }

        let path = Path::new("/app/langram_models");
        File::open(path.join(BinStorage::FILE_NAME)).map_err(ModelsStorageError::ModelsFileOpen)
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
        let mut file_storage = BinStorage::default();

        for (l, m) in input {
            file_storage.add(l.into_str().to_owned(), m);
        }
        file_storage.reorder();

        let bytes = file_storage.to_bytes().unwrap();
        let mmap = mmap_from_bytes(&bytes).unwrap();

        Self::from_mmap(mmap).unwrap()
    }
}

#[cfg(test)]
fn mmap_from_bytes(data: &[u8]) -> std::io::Result<Mmap> {
    let mut mmap = memmap2::MmapMut::map_anon(data.len())?;
    mmap.copy_from_slice(data);
    mmap.make_read_only()
}

#[derive(Error, Debug)]
pub enum ModelsStorageError {
    #[error("Current exe error")]
    CurrentExe(#[source] io::Error),
    #[error("Models file open error")]
    ModelsFileOpen(#[source] io::Error),
    #[error("Mmap error")]
    Mmap(#[source] io::Error),
    #[error("Rkyv access error")]
    RkyvAccess(#[from] rkyv::rancor::Error),
    #[error("Langram models hash {0:X} is incompatible, please recompile models!")]
    ModelsHash(u64),
}
