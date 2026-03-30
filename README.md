# lifelog-audio-compression

Opinionated speech-first audio compression for lifelog archives.

Current default:

- mono
- 16 kHz
- Opus
- one input audio file -> one output `audio-bundle`

The tool is intentionally simple:

- keep the original local master somewhere else
- emit one compressed upload-friendly derivative
- preserve intrinsic source-audio metadata in `bundle.json`

Commands:

```bash
cargo run -- spec
cargo run -- compress /path/to/input.wav /path/to/output-bundle
cargo run -- batch /path/to/audio-folder /path/to/output-root
```

Library entry points:

- `compress_to_dir(input, output_dir)`
- `compress_directory_to_dir(input_root, output_root)`
- `load_bundle_metadata(bundle_dir)`
