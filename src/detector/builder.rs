use super::{Detector, ModelsStorage, NgramSize};
use crate::ngram_size::{NgramSizes, NgramSizesTrait};
use ::core::hash::BuildHasher;
use ::std::{collections::HashSet, hash::DefaultHasher};
use alphabet_detector::ScriptLanguage;

pub struct DummyBuildHasher;
impl BuildHasher for DummyBuildHasher {
    type Hasher = DefaultHasher;
    #[inline(never)]
    fn build_hasher(&self) -> Self::Hasher {
        unreachable!();
    }
}

pub trait RealHasher: Sized + BuildHasher + Default {}
impl<T: BuildHasher + Default> RealHasher for T {}

#[derive(Clone, Debug)]
pub struct DetectorBuilder<'m, H: BuildHasher> {
    pub(super) models_storage: &'m ModelsStorage,
    pub(super) languages: HashSet<ScriptLanguage, H>,
    pub(super) long_text_minlen: usize,
    pub(super) long_text_ngrams: NgramSizes,
    pub(super) short_text_ngrams: NgramSizes,
}

impl<'m> DetectorBuilder<'m, DummyBuildHasher> {
    /// Will have all languages, all ngrams enabled if none selected
    #[inline]
    pub fn new(models_storage: &'m ModelsStorage) -> Self {
        Self {
            models_storage,
            languages: HashSet::with_hasher(DummyBuildHasher),
            long_text_minlen: 120,
            long_text_ngrams: NgramSizes::new_const(),
            short_text_ngrams: NgramSizes::new_const(),
        }
    }

    /// Build with all languages
    #[inline]
    // pub fn build<H2: RealHasher>(self) -> Detector<'m, H2> {
    pub fn build(self) -> Detector<'m, ahash::RandomState> {
        Detector::new(self.languages(ScriptLanguage::all().collect()))
    }
}

impl<'m, H: RealHasher> DetectorBuilder<'m, H> {
    #[inline]
    pub fn build(self) -> Detector<'m, H> {
        Detector::new(self)
    }
}

impl<'m, H: BuildHasher> DetectorBuilder<'m, H> {
    /// Change languages
    #[inline]
    pub fn languages<H2: RealHasher>(
        self,
        languages: HashSet<ScriptLanguage, H2>,
    ) -> DetectorBuilder<'m, H2> {
        DetectorBuilder {
            models_storage: self.models_storage,
            languages,
            long_text_minlen: self.long_text_minlen,
            long_text_ngrams: self.long_text_ngrams,
            short_text_ngrams: self.short_text_ngrams,
        }
    }

    /// Min text length (in chars, excluding word separators) for
    /// switching from short ngrams to long ngrams
    #[inline]
    pub fn minlen(mut self, long_text_minlen: usize) -> Self {
        self.long_text_minlen = long_text_minlen;
        self
    }

    /// Select ngrams for text length >= minlen (in chars, excluding word separators)
    #[inline]
    pub fn long_ngrams(mut self, ngrams: impl Iterator<Item = NgramSize>) -> Self {
        self.long_text_ngrams = NgramSizes::new_merged(ngrams);
        self
    }

    /// Select ngrams for text length < minlen (in chars, excluding word separators)
    #[inline]
    pub fn short_ngrams(mut self, ngrams: impl Iterator<Item = NgramSize>) -> Self {
        self.short_text_ngrams = NgramSizes::new_merged(ngrams);
        self
    }

    /// Add long text ngrams. Starts with empty ngrams
    #[inline]
    pub fn long_ngrams_add(mut self, ngrams: impl Iterator<Item = NgramSize>) -> Self {
        self.long_text_ngrams.merge(ngrams);
        self
    }

    /// Add short text ngrams. Starts with empty ngrams
    #[inline]
    pub fn short_ngrams_add(mut self, ngrams: impl Iterator<Item = NgramSize>) -> Self {
        self.short_text_ngrams.merge(ngrams);
        self
    }

    /// Faster, but lower accuracy
    #[inline]
    pub fn max_trigrams(mut self) -> Self {
        self.long_text_ngrams =
            NgramSizes::new_merged([NgramSize::Tri, NgramSize::Word].into_iter());
        self.short_text_ngrams = NgramSizes::new_merged(
            [
                NgramSize::Uni,
                NgramSize::Bi,
                NgramSize::Tri,
                NgramSize::Word,
            ]
            .into_iter(),
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::DetectorBuilder;
    use crate::{ModelsStorage, ScriptLanguage};
    use ::std::collections::HashSet;
    use ahash::AHashSet;
    use strum::EnumCount;

    #[test]
    fn test_build() {
        let storage = ModelsStorage::default();
        let detector = DetectorBuilder::new(&storage).build();
        assert_eq!(detector.languages.len(), ScriptLanguage::COUNT);

        let _detector = DetectorBuilder::new(&storage)
            .languages(AHashSet::new().into())
            .build();

        let _detector = DetectorBuilder::new(&storage)
            .languages(HashSet::new())
            .build();
    }

    #[test]
    fn test_empty_ngrams() {
        let storage = ModelsStorage::default();
        let builder = DetectorBuilder::new(&storage)
            .long_ngrams([].into_iter())
            .short_ngrams([].into_iter());
        assert!(builder.long_text_ngrams.is_empty());
        assert!(builder.short_text_ngrams.is_empty());

        let detector = builder.build();
        assert!(!detector.long_text_ngrams.is_empty());
        assert!(!detector.short_text_ngrams.is_empty());
    }

    #[test]
    fn test_hasher_change() {
        let storage = ModelsStorage::default();
        let builder = DetectorBuilder::new(&storage);
        let builder = builder.languages(HashSet::new());
        let builder = builder.languages(AHashSet::new().into());
        let _detector = builder.build();
    }
}
