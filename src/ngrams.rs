use crate::NGRAM_MAX_SIZE;
use ahash::AHashSet;
use arraystring::{typenum::U20, ArrayString};

pub(crate) type NgramString = ArrayString<U20>;

pub(crate) fn prepare_ngrams<'a>(
    words: impl Iterator<Item = &'a [char]>,
    ngram_length: usize,
) -> Vec<NgramString> {
    debug_assert!(
        (1..=NGRAM_MAX_SIZE).contains(&ngram_length),
        "ngram length {ngram_length} is not in range 1..={NGRAM_MAX_SIZE}"
    );

    let mut ngrams_tmp = AHashSet::new();
    let mut ngrams = Vec::new();

    for word in words {
        for ngram in word.windows(ngram_length) {
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
    use crate::NGRAM_MAX_SIZE;

    #[test]
    fn test_ngram_string_size() {
        let max_ngram = [char::MAX; NGRAM_MAX_SIZE];
        NgramString::try_from_chars(max_ngram).unwrap();
    }
}
