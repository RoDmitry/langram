use super::{Detector, ModelsStorage, NgramSize};
use crate::ngram_size::{NgramSizes, NgramSizesTrait};
use alphabet_detector::{ScriptLanguage, ScriptLanguageIter};
use strum::IntoEnumIterator;

#[derive(Clone, Debug)]
pub struct DetectorBuilder<'m, L>
where
    L: IntoIterator<Item = ScriptLanguage>,
{
    pub(super) models_storage: &'m ModelsStorage<'m>,
    pub(super) languages: L,
    pub(super) long_text_minlen: usize,
    pub(super) long_text_ngrams: NgramSizes,
    pub(super) short_text_ngrams: NgramSizes,
}

impl<'m> DetectorBuilder<'m, ScriptLanguageIter> {
    /// Will have all languages, all ngrams enabled if none selected
    #[inline]
    pub fn new(models_storage: &'m ModelsStorage) -> Self {
        Self {
            models_storage,
            languages: ScriptLanguage::iter(),
            long_text_minlen: 120,
            long_text_ngrams: NgramSizes::new_const(),
            short_text_ngrams: NgramSizes::new_const(),
        }
    }
}

impl<'m, L> DetectorBuilder<'m, L>
where
    L: IntoIterator<Item = ScriptLanguage>,
{
    #[inline]
    pub fn build(self) -> Detector<'m> {
        Detector::new(self)
    }

    /// Change languages
    #[inline]
    pub fn languages<L2: IntoIterator<Item = ScriptLanguage>>(
        self,
        languages: L2,
    ) -> DetectorBuilder<'m, L2> {
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
        let storage = ModelsStorage::new().unwrap();
        let detector = DetectorBuilder::new(&storage).build();
        assert_eq!(detector.languages.len(), ScriptLanguage::COUNT);

        let _detector = DetectorBuilder::new(&storage).languages([]).build();

        let _detector = DetectorBuilder::new(&storage)
            .languages(HashSet::new())
            .build();
    }

    #[test]
    fn test_empty_ngrams() {
        let storage = ModelsStorage::new().unwrap();
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
        let storage = ModelsStorage::new().unwrap();
        let builder = DetectorBuilder::new(&storage);
        let builder = builder.languages(HashSet::new());
        let builder = builder.languages(AHashSet::new());
        let _detector = builder.build();
    }
}
