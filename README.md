# Langram - the most accurate language detection library

[![Crate](https://img.shields.io/crates/v/langram.svg)](https://crates.io/crates/langram)
[![API](https://docs.rs/langram/badge.svg)](https://docs.rs/langram)

## 308 ScriptLanguages (187 models + 121 single language scripts)

> One language can be written in multiple scripts, so it will be detected as a different [`ScriptLanguage`](https://docs.rs/alphabet_detector/latest/alphabet_detector/enum.ScriptLanguage.html) (language + script)

Uses [`alphabet_detector`](https://github.com/RoDmitry/alphabet_detector) as a word separator + language prefilter.

Based on char (not word) [n-gram language model](https://en.wikipedia.org/wiki/Word_n-gram_language_model) modified algorithm.

This library is a complete rewrite of Lingua: 5x faster, more accuracy, more languages, etc.

[Accuracy report](https://github.com/RoDmitry/lang_detectors_compare/blob/main/accuracy/langram.csv)

[Comparison with other language detectors](https://github.com/RoDmitry/lang_detectors_compare)

### Setup

To use it, you need to patch `langram_models` in `Cargo.toml`:

* From Git:
```
[patch.crates-io]
langram_models = { git = "https://github.com/RoDmitry/langram_models.git" }
```

* From predownloaded copy:
```
[patch.crates-io]
langram_models = { path = "../langram_models" }
```
Which is more advanced and allows you to remove model ngrams, so that final executable would be lighter.
