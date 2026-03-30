# Changelog

## 0.1.1 - 2026-03-30

Metadata preservation patch release.

Highlights:

- keep raw source metadata artifacts in each audio bundle:
  - `source-ffprobe.json`
  - `source-mdls.txt`
  - `source-xattrs.txt`
- expose the metadata artifact paths in `bundle.json`
- update CLI/docs to make metadata preservation explicit

## 0.1.0 - 2026-03-30

Initial public release.

Highlights:

- opinionated `compress` command for one audio file to one audio bundle
- simple `batch` command for recursive directory compression
- standard output format:
  - mono
  - 16 kHz
  - Opus
- Rust library entry points:
  - `compress_to_dir`
  - `compress_directory_to_dir`
  - `load_bundle_metadata`
