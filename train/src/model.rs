use ahash::AHashMap;
use fraction::GenericFraction;
use langram::{FileModel, Fraction};

#[derive(Debug)]
pub(crate) struct TrainingModel<'t> {
    pub(crate) absolute_frequencies: AHashMap<&'t [char], usize>,
    lower_ngram_absolute_frequencies: AHashMap<&'t [char], usize>,
}

impl<'t> TrainingModel<'t> {
    pub(crate) fn new(
        words_chars: &'t [Vec<char>],
        lower_ngram_absolute_frequencies: AHashMap<&'t [char], usize>,
    ) -> Self {
        let mut absolute_frequencies = AHashMap::new();
        for chars in words_chars.iter() {
            *absolute_frequencies.entry(chars.as_ref()).or_default() += 1;
        }

        Self {
            absolute_frequencies,
            lower_ngram_absolute_frequencies,
        }
    }

    pub(crate) fn new_windows(
        words_chars: &'t [Vec<char>],
        lower_ngram_absolute_frequencies: AHashMap<&'t [char], usize>,
        ngram_length: usize,
    ) -> Self {
        let mut absolute_frequencies = AHashMap::new();
        for chars in words_chars.iter() {
            for ngram in chars.windows(ngram_length) {
                *absolute_frequencies.entry(ngram).or_default() += 1;
            }
        }
        // let min_count = absolute_frequencies.values().sum::<usize>() / 10_000_000;
        // absolute_frequencies.retain(|_, c| *c > min_count);

        Self {
            absolute_frequencies,
            lower_ngram_absolute_frequencies,
        }
    }

    fn compute_relative_frequencies(&self) -> AHashMap<GenericFraction<usize>, Vec<&'t [char]>> {
        let total_count = self.absolute_frequencies.values().sum::<usize>();
        let mut ngram_probabilities: AHashMap<GenericFraction<usize>, Vec<_>> = AHashMap::new();

        for (&ngram, &frequency) in self.absolute_frequencies.iter() {
            let denominator = if self.lower_ngram_absolute_frequencies.is_empty() {
                total_count
            } else {
                let Some(&start_ngram_abs_fr) = self
                    .lower_ngram_absolute_frequencies
                    .get(&ngram[..ngram.len() - 1])
                else {
                    continue;
                };
                let Some(&end_ngram_abs_fr) =
                    self.lower_ngram_absolute_frequencies.get(&ngram[1..])
                else {
                    continue;
                };
                start_ngram_abs_fr.min(end_ngram_abs_fr)
            };
            let fract = GenericFraction::<usize>::new(frequency, denominator);
            ngram_probabilities.entry(fract).or_default().push(ngram);
        }

        ngram_probabilities
    }

    pub(crate) fn to_file_model(&self, join: &[char]) -> FileModel {
        let relative_frequencies = self.compute_relative_frequencies();
        let mut sorted: Vec<_> = relative_frequencies.into_iter().collect();
        sorted.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        let mut lang_model: FileModel = (self.absolute_frequencies.len(), Default::default());
        for (gf, mut ngrams) in sorted {
            ngrams.sort_unstable();
            lang_model.1.insert_unchecked(
                Fraction::from(gf),
                // ngrams.into_iter().flat_map(|v| v.iter()).collect(),
                itertools::Itertools::intersperse(ngrams.into_iter(), join)
                    .flat_map(|v| v.iter())
                    .collect(),
            );
        }

        lang_model
    }

    /*pub(crate) fn to_match(self, file_path: &Path) -> io::Result<()> {
        let mut sorted: Vec<_> = self.relative_frequencies.unwrap().into_iter().collect();
        sorted.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        if let Some(parent) = file_path.parent() {
            create_dir_all(parent)?;
        }
        let mut file = File::create(file_path)?;
        file.write_all(b"#![cfg_attr(rustfmt,rustfmt_skip)]\n")?;
        if self.ngram_length == 1 {
            file.write_all(b"pub(super) fn prob(c:char) -> f64 {\nmatch c {\n")?;
        } else {
            file.write_all(b"pub(super) fn prob(g:&[char;")?;
            file.write_all(self.ngram_length.to_string().as_bytes())?;
            file.write_all(b"]) -> f64 {\nmatch g {\n")?;
        }

        for (fraction, ngrams) in sorted {
            if self.ngram_length == 1 {
                file.write_all(b"'")?;
                file.write_all(
                    ngrams
                        .into_iter()
                        .map(|n| {
                            n.chars()
                                .map(|c| {
                                    if c == '\'' {
                                        "\\'".to_owned()
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .next()
                                .unwrap()
                        })
                        .join("'|'")
                        .as_bytes(),
                )?;
                file.write_all(b"'=>")?;
            } else {
                file.write_all(b"&['")?;
                file.write_all(
                    ngrams
                        .into_iter()
                        .map(|n| {
                            n.chars()
                                .map(|c| {
                                    if c == '\'' {
                                        "\\'".to_owned()
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .join("','")
                        })
                        .join("']|&['")
                        .as_bytes(),
                )?;
                file.write_all(b"']=>")?;
            }

            let numer = fraction.numer().unwrap();
            let denom = fraction.denom().unwrap();
            if numer == denom {
                file.write_all(b"1.0,\n")?;
            } else {
                file.write_all(numer.to_string().as_bytes())?;
                file.write_all(b".0/")?;
                file.write_all(denom.to_string().as_bytes())?;
                file.write_all(b".0,\n")?;
            }
        }
        file.write_all(b"_=>0.0,\n}\n}")
    }*/
}
