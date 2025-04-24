# Langram train models

[![Crate](https://img.shields.io/crates/v/langram_train.svg)](https://crates.io/crates/langram_train)
[![API](https://docs.rs/langram_train/badge.svg)](https://docs.rs/langram_train)

Used [OpenLID](https://github.com/laurieburchell/open-lid-dataset) (201 languages).

Unpacked with `pigz -dc ../lid201-data.tsv.gz | awk -F"\t" '{gsub(/_/, "", $2); print $1 > $2}'`.
Renamed `korHang` to `korKore`, `zho` to `cmn`, `est` to `ekk`, `tgl` to `fil`, `grn` to `gug`, `kon` to `ktu`.
