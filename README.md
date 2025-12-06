# Langram - the most accurate language detection library

[![Crate](https://img.shields.io/crates/v/langram.svg)](https://crates.io/crates/langram)
[![API](https://docs.rs/langram/badge.svg)](https://docs.rs/langram)

## 321 ScriptLanguages (187 models + 134 single language scripts)

### Usage examples in [docs.rs](https://docs.rs/langram).

> One language can be written in multiple scripts, so it will be detected as a different [`ScriptLanguage`](https://docs.rs/langram/latest/langram/enum.ScriptLanguage.html) (language + script)

Uses [`alphabet_detector`](https://github.com/RoDmitry/alphabet_detector) as a word separator + language prefilter.

Based on chars (1 - 5) and 1 word [n-gram language model](https://en.wikipedia.org/wiki/Word_n-gram_language_model) modified algorithm.

RAM requirements are low, but it may take up to the provided models binary file's size, but this memory is shared (Virtual space, Mmap), so it's not required to have that amount of RAM available.
But if it won't be able to cache the whole models file in RAM, it's speed will be affected.

This library is a complete rewrite of Lingua: much faster, more accuracy, more languages, etc.

Also more accurate than Whatlang or Whichlang. More info at the [Comparison with other language detectors](https://github.com/RoDmitry/lang_detectors_compare).

To better understand the accuracy of different modes, look into the [Accuracy report](https://github.com/RoDmitry/lang_detectors_compare/blob/main/accuracy/langram.csv).

## Setup

To use this library, you need a binary models file, which must be placed near the executable, or set `LANGRAM_MODELS_PATH`.

It can be:

* Downloaded from [langram_models releases](https://github.com/RoDmitry/langram_models/releases);

* Built (recommened if big-endian target) [langram_models](https://github.com/RoDmitry/langram_models). Which is more advanced and allows you to remove model ngrams, and recompile, so that models binary would be lighter.
