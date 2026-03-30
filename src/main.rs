use lifelog_audio_compression::{
    CompressDefaults, CompressRequest, compress, compress_directory_to_dir,
};
use std::env;
use std::path::PathBuf;

fn print_usage() {
    println!(
        r#"lifelog-audio-compression

An opinionated utility for turning one source audio file into one speech-first audio bundle.

Commands:
  compress <input-audio> <output-dir>
    Intended default:
      - mono
      - 16 kHz
      - Opus
      - one input audio -> one output bundle

  batch <input-dir> <output-root>
    Recursively compress every supported audio file under a directory.

  spec
    Print the current bundle design summary.

  help
    Show this message.
"#
    );
}

fn print_spec() {
    let defaults = CompressDefaults::default();
    println!(
        r#"Current audio-bundle v1 direction:

- canonical representation: one Opus file + bundle.json
- codec: opus
- extension: {ext}
- sample rate: {rate} Hz
- channels: {channels}
- bitrate: {bitrate}
- one input audio -> one output bundle -> one upload unit
"#,
        ext = defaults.extension,
        rate = defaults.sample_rate_hz,
        channels = defaults.channels,
        bitrate = defaults.bitrate,
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        print_usage();
        return;
    };

    match command {
        "help" | "--help" | "-h" => print_usage(),
        "spec" => print_spec(),
        "compress" => {
            if args.len() != 3 {
                eprintln!("usage: lifelog-audio-compression compress <input-audio> <output-dir>");
                std::process::exit(2);
            }
            let request = CompressRequest::new(PathBuf::from(&args[1]), PathBuf::from(&args[2]));
            match compress(&request) {
                Ok(result) => println!("{}", result.bundle_dir.display()),
                Err(message) => {
                    eprintln!("{message}");
                    std::process::exit(2);
                }
            }
        }
        "batch" => {
            if args.len() != 3 {
                eprintln!("usage: lifelog-audio-compression batch <input-dir> <output-root>");
                std::process::exit(2);
            }
            match compress_directory_to_dir(PathBuf::from(&args[1]), PathBuf::from(&args[2])) {
                Ok(result) => {
                    println!("{}\nitems={}", result.output_root.display(), result.item_count);
                }
                Err(message) => {
                    eprintln!("{message}");
                    std::process::exit(2);
                }
            }
        }
        _ => {
            eprintln!("unknown command: {command}");
            print_usage();
            std::process::exit(2);
        }
    }
}
