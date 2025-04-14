# Langram train models

Used [OpenLID](https://github.com/laurieburchell/open-lid-dataset) (201 languages).

Unpacked with `pigz -dc ../lid201-data.tsv.gz | awk -F"\t" '{gsub(/_/, "", $2); print $1 > $2}'`.
Renamed `korHang` to `korKore`.
