use ahash::AHashMap;
use fraction::GenericFraction;
use langram::{FileModel, Fraction};

#[derive(Debug)]
pub(crate) struct TrainingModel<'t> {
    ngram_length: usize,
    pub(crate) absolute_frequencies: AHashMap<&'t [char], usize>,
    lower_ngram_absolute_frequencies: AHashMap<&'t [char], usize>,
}

impl<'t> TrainingModel<'t> {
    pub(crate) fn from_text(
        words_chars: &'t [Vec<char>],
        ngram_length: usize,
        lower_ngram_absolute_frequencies: AHashMap<&'t [char], usize>,
    ) -> Self {
        let mut absolute_frequencies = AHashMap::new();
        for chars in words_chars.iter() {
            if chars.len() < ngram_length {
                continue;
            }

            for i in 0..=chars.len() - ngram_length {
                let ngram = &chars[i..i + ngram_length];
                *absolute_frequencies.entry(ngram).or_default() += 1;
            }
        }

        Self {
            ngram_length,
            absolute_frequencies,
            lower_ngram_absolute_frequencies,
        }
    }

    fn compute_relative_frequencies(&self) -> AHashMap<GenericFraction<usize>, Vec<&'t [char]>> {
        let total_ngram_frequency = self.absolute_frequencies.values().sum::<usize>();
        let mut ngram_probabilities: AHashMap<GenericFraction<usize>, Vec<_>> = AHashMap::new();

        for (&ngram, frequency) in self.absolute_frequencies.iter() {
            let denominator =
                if self.ngram_length == 1 || self.lower_ngram_absolute_frequencies.is_empty() {
                    total_ngram_frequency
                } else {
                    let start_ngram_abs_fr = *self
                        .lower_ngram_absolute_frequencies
                        .get(&ngram[..ngram.len() - 1])
                        .unwrap();
                    let end_ngram_abs_fr = *self
                        .lower_ngram_absolute_frequencies
                        .get(&ngram[1..])
                        .unwrap();
                    start_ngram_abs_fr.min(end_ngram_abs_fr)
                };
            let fract = GenericFraction::<usize>::new(*frequency, denominator);
            ngram_probabilities.entry(fract).or_default().push(ngram);
        }

        ngram_probabilities
    }

    pub(crate) fn to_file_model(&self) -> FileModel {
        let relative_frequencies = self.compute_relative_frequencies();
        let mut sorted: Vec<_> = relative_frequencies.into_iter().collect();
        sorted.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        let mut lang_model = FileModel::default();
        for (gf, mut ngrams) in sorted {
            ngrams.sort_unstable();
            lang_model.insert_unchecked(
                Fraction::from(gf),
                ngrams.into_iter().flat_map(|v| v.iter()).collect(),
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
