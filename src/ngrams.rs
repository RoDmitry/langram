use crate::NgramSize;
use arraystring::{typenum::U20, ArrayString};
use debug_unsafe::arraystring::ArrayStringFrom;
use rustc_hash::FxHashSet;

pub(crate) type NgramString = ArrayString<U20>;

pub(crate) struct NgramIterator<'w, I>
where
    I: Iterator<Item = &'w [char]>,
{
    ngrams: I,
    seen: FxHashSet<&'w [char]>,
}

pub(crate) fn ngram_iterator<'w>(
    words_iter: impl Iterator<Item = &'w [char]>,
    ngram_size: NgramSize,
) -> NgramIterator<'w, impl Iterator<Item = &'w [char]>> {
    let ngrams = words_iter.flat_map(move |w| w.windows(ngram_size as usize + 1));

    NgramIterator {
        ngrams,
        seen: Default::default(),
    }
}

impl<'w, I> Iterator for NgramIterator<'w, I>
where
    I: Iterator<Item = &'w [char]>,
{
    type Item = NgramString;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let ngram = self.ngrams.next()?;
            if self.seen.insert(ngram) {
                return Some(NgramString::from_chars_safe_unchecked(
                    ngram.iter().copied(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NgramString;
    use crate::ngram_size::NGRAM_MAX_LEN;

    #[test]
    fn test_ngram_string_size() {
        let max_ngram = [char::MAX; NGRAM_MAX_LEN];
        NgramString::try_from_chars(max_ngram).unwrap();
    }
}
