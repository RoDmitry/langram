use crate::detector::NgramSize;
use ahash::AHashSet;
use arraystring::{typenum::U20, ArrayString};

pub(crate) type NgramString = ArrayString<U20>;

pub(crate) fn prepare_ngrams<'a>(
    words: impl Iterator<Item = &'a [char]>,
    ngram_size: NgramSize,
) -> Vec<NgramString> {
    let mut ngrams_tmp = AHashSet::new();
    let mut ngrams = Vec::new();

    for word in words {
        for ngram in word.windows(ngram_size as usize + 1) {
            if ngrams_tmp.insert(ngram) {
                ngrams.push(NgramString::try_from_chars(ngram.iter().copied()).unwrap());
            }
        }
    }

    ngrams
}

#[cfg(test)]
mod tests {
    use super::NgramString;
    use crate::detector::NGRAM_MAX_LEN;

    #[test]
    fn test_ngram_string_size() {
        let max_ngram = [char::MAX; NGRAM_MAX_LEN];
        NgramString::try_from_chars(max_ngram).unwrap();
    }
}
