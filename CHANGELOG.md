# Changelog

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
