las-rs
======

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/gadomski/las-rs/continuous-integration.yml?branch=main&style=for-the-badge)](https://github.com/gadomski/las-rs/actions/workflows/continuous-integration.yml)
[![Crates.io](https://img.shields.io/crates/v/las?style=for-the-badge)](https://crates.io/crates/las)
[![docs.rs](https://img.shields.io/docsrs/las?style=for-the-badge)](https://docs.rs/las/latest/las/)
![Crates.io](https://img.shields.io/crates/l/las?style=for-the-badge)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg?style=for-the-badge)](./CODE_OF_CONDUCT)

Read and write [ASPRS las files](http://www.asprs.org/Committee-General/LASer-LAS-File-Format-Exchange-Activities.html) natively with rust.

```toml
[dependencies]
las = "0.8"
```

To include [laz](https://laszip.org/) support:

```toml
[dependencies]
las = { version = "0.8", features = ["laz"] }
```

To include `laz` support with parallel compression/decompression
to enhance speed (`laz-parallel` implies `laz` so you don't need to specify both):

```toml
[dependencies]
las = { version = "0.8", features = ["laz-parallel"] }
```
