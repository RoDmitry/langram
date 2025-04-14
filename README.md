# Langram - the most accurate language detection library

[![Crate](https://img.shields.io/crates/v/langram.svg)](https://crates.io/crates/langram)
[![API](https://docs.rs/langram/badge.svg)](https://docs.rs/langram)

## Over 200 languages

Uses [`alphabet_detector`](https://github.com/RoDmitry/alphabet_detector) as a word separator + language prefilter.

Based on char (not word) [n-gram language model](https://en.wikipedia.org/wiki/Word_n-gram_language_model) modified algorithm.

This library is a complete rewrite of Lingua: more languages, more accuracy, faster, etc.

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
