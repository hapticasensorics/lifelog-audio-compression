use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressRequest {
    pub input: PathBuf,
    pub output_dir: PathBuf,
}

impl CompressRequest {
    pub fn new(input: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> Self {
        Self {
            input: input.as_ref().to_path_buf(),
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressDefaults {
    pub sample_rate_hz: u32,
    pub channels: u32,
    pub codec: &'static str,
    pub bitrate: &'static str,
    pub extension: &'static str,
}

impl Default for CompressDefaults {
    fn default() -> Self {
        Self {
            sample_rate_hz: 16_000,
            channels: 1,
            codec: "libopus",
            bitrate: "24k",
            extension: "ogg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressResult {
    pub bundle_dir: PathBuf,
    pub audio_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItemResult {
    pub input: PathBuf,
    pub bundle_dir: PathBuf,
    pub audio_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCompressResult {
    pub output_root: PathBuf,
    pub item_count: usize,
    pub items: Vec<BatchItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProducerMetadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompressionMetadata {
    pub codec: String,
    pub container_extension: String,
    pub sample_rate_hz: u32,
    pub channels: u32,
    pub bitrate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceAudioMetadata {
    pub source_relpath: String,
    pub file_size_bytes: u64,
    pub container_format: String,
    pub audio_codec: String,
    pub duration_ms: u64,
    pub sample_rate_hz: u32,
    pub channels: u32,
    pub channel_layout: Option<String>,
    pub bits_per_sample: u32,
    pub creation_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputAudioMetadata {
    pub audio_relpath: String,
    pub file_size_bytes: u64,
    pub duration_ms: u64,
    pub sample_rate_hz: u32,
    pub channels: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BundleMetadata {
    pub format: String,
    pub format_version: u32,
    pub bundle_id: String,
    pub created_at_unix_ms: u128,
    pub producer: ProducerMetadata,
    pub compression: CompressionMetadata,
    pub source_audio: SourceAudioMetadata,
    pub output_audio: OutputAudioMetadata,
    pub metadata_artifacts: MetadataArtifacts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetadataArtifacts {
    pub ffprobe_relpath: String,
    pub mdls_relpath: String,
    pub xattrs_relpath: String,
}

fn run_command(program: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run `{program}`: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("`{program}` failed: {stderr}"));
    }
    Ok(output.stdout)
}

fn run_command_allow_failure(program: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run `{program}`: {err}"))?;
    Ok(output.stdout)
}

fn ffprobe_json(input: &Path) -> Result<Value, String> {
    let stdout = run_command(
        "ffprobe",
        &[
            "-v",
            "error",
            "-show_entries",
            "format=format_name,duration,size:format_tags=creation_time:stream=index,codec_type,codec_name,sample_rate,channels,channel_layout,bits_per_sample",
            "-of",
            "json",
            input
                .to_str()
                .ok_or_else(|| format!("non-utf8 input path: {}", input.display()))?,
        ],
    )?;
    serde_json::from_slice(&stdout).map_err(|err| format!("failed to parse ffprobe json: {err}"))
}

fn ffprobe_json_bytes(input: &Path) -> Result<Vec<u8>, String> {
    run_command(
        "ffprobe",
        &[
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-show_chapters",
            "-show_programs",
            "-print_format",
            "json",
            input
                .to_str()
                .ok_or_else(|| format!("non-utf8 input path: {}", input.display()))?,
        ],
    )
}

fn write_source_metadata_artifacts(input: &Path, output_dir: &Path) -> Result<MetadataArtifacts, String> {
    let input_str = input
        .to_str()
        .ok_or_else(|| format!("non-utf8 input path: {}", input.display()))?;

    let ffprobe_json = ffprobe_json_bytes(input)?;
    fs::write(output_dir.join("source-ffprobe.json"), ffprobe_json)
        .map_err(|err| format!("failed to write ffprobe metadata: {err}"))?;

    let mdls_text = run_command("mdls", &[input_str])?;
    fs::write(output_dir.join("source-mdls.txt"), mdls_text)
        .map_err(|err| format!("failed to write mdls metadata: {err}"))?;

    let xattrs_text = run_command_allow_failure("xattr", &["-l", input_str])?;
    fs::write(output_dir.join("source-xattrs.txt"), xattrs_text)
        .map_err(|err| format!("failed to write xattr metadata: {err}"))?;

    Ok(MetadataArtifacts {
        ffprobe_relpath: "source-ffprobe.json".to_string(),
        mdls_relpath: "source-mdls.txt".to_string(),
        xattrs_relpath: "source-xattrs.txt".to_string(),
    })
}

fn extract_source_audio_metadata(input: &Path) -> Result<SourceAudioMetadata, String> {
    let probe = ffprobe_json(input)?;
    let streams = probe["streams"]
        .as_array()
        .ok_or_else(|| "ffprobe streams missing".to_string())?;
    let audio_stream = streams
        .iter()
        .find(|stream| stream["codec_type"].as_str() == Some("audio"))
        .ok_or_else(|| "no audio stream found".to_string())?;

    let format = &probe["format"];
    let duration_ms = (format["duration"]
        .as_str()
        .ok_or_else(|| "format duration missing".to_string())?
        .parse::<f64>()
        .map_err(|err| format!("bad duration: {err}"))?
        * 1000.0)
        .round() as u64;
    let file_size_bytes = format["size"]
        .as_str()
        .ok_or_else(|| "format size missing".to_string())?
        .parse::<u64>()
        .map_err(|err| format!("bad size: {err}"))?;

    Ok(SourceAudioMetadata {
        source_relpath: input.to_string_lossy().to_string(),
        file_size_bytes,
        container_format: format["format_name"]
            .as_str()
            .unwrap_or("unknown")
            .split(',')
            .next()
            .unwrap_or("unknown")
            .to_string(),
        audio_codec: audio_stream["codec_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        duration_ms,
        sample_rate_hz: audio_stream["sample_rate"]
            .as_str()
            .unwrap_or("0")
            .parse::<u32>()
            .unwrap_or(0),
        channels: audio_stream["channels"].as_u64().unwrap_or(0) as u32,
        channel_layout: audio_stream["channel_layout"].as_str().map(ToOwned::to_owned),
        bits_per_sample: audio_stream["bits_per_sample"].as_u64().unwrap_or(0) as u32,
        creation_time: format["tags"]["creation_time"]
            .as_str()
            .map(ToOwned::to_owned),
    })
}

fn extract_output_audio_metadata(output: &Path) -> Result<OutputAudioMetadata, String> {
    let probe = ffprobe_json(output)?;
    let streams = probe["streams"]
        .as_array()
        .ok_or_else(|| "ffprobe streams missing".to_string())?;
    let audio_stream = streams
        .iter()
        .find(|stream| stream["codec_type"].as_str() == Some("audio"))
        .ok_or_else(|| "no audio stream found".to_string())?;
    let format = &probe["format"];
    let duration_ms = (format["duration"]
        .as_str()
        .ok_or_else(|| "format duration missing".to_string())?
        .parse::<f64>()
        .map_err(|err| format!("bad duration: {err}"))?
        * 1000.0)
        .round() as u64;
    let file_size_bytes = format["size"]
        .as_str()
        .ok_or_else(|| "format size missing".to_string())?
        .parse::<u64>()
        .map_err(|err| format!("bad size: {err}"))?;
    Ok(OutputAudioMetadata {
        audio_relpath: format!("audio.{}", CompressDefaults::default().extension),
        file_size_bytes,
        duration_ms,
        sample_rate_hz: audio_stream["sample_rate"]
            .as_str()
            .unwrap_or("0")
            .parse::<u32>()
            .unwrap_or(0),
        channels: audio_stream["channels"].as_u64().unwrap_or(0) as u32,
    })
}

fn bundle_id() -> Result<(String, u128), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?;
    let millis = now.as_millis();
    Ok((format!("ab_{millis:x}"), millis))
}

pub fn compress(request: &CompressRequest) -> Result<CompressResult, String> {
    if !request.input.exists() {
        return Err(format!("input does not exist: {}", request.input.display()));
    }
    if !request.input.is_file() {
        return Err(format!("input is not a file: {}", request.input.display()));
    }

    let defaults = CompressDefaults::default();
    if request.output_dir.exists() {
        fs::remove_dir_all(&request.output_dir)
            .map_err(|err| format!("failed to remove {}: {err}", request.output_dir.display()))?;
    }
    fs::create_dir_all(&request.output_dir)
        .map_err(|err| format!("failed to create {}: {err}", request.output_dir.display()))?;

    let metadata_artifacts = write_source_metadata_artifacts(&request.input, &request.output_dir)?;
    let source_audio = extract_source_audio_metadata(&request.input)?;
    let output_audio_path = request
        .output_dir
        .join(format!("audio.{}", defaults.extension));

    run_command(
        "ffmpeg",
        &[
            "-y",
            "-v",
            "error",
            "-i",
            request
                .input
                .to_str()
                .ok_or_else(|| format!("non-utf8 input path: {}", request.input.display()))?,
            "-vn",
            "-ac",
            &defaults.channels.to_string(),
            "-ar",
            &defaults.sample_rate_hz.to_string(),
            "-c:a",
            defaults.codec,
            "-b:a",
            defaults.bitrate,
            "-application",
            "voip",
            output_audio_path.to_str().ok_or_else(|| {
                format!("non-utf8 output path: {}", output_audio_path.display())
            })?,
        ],
    )?;

    let output_audio = extract_output_audio_metadata(&output_audio_path)?;
    let (bundle_id, created_at_unix_ms) = bundle_id()?;
    let bundle = BundleMetadata {
        format: "audio-bundle".to_string(),
        format_version: 1,
        bundle_id,
        created_at_unix_ms,
        producer: ProducerMetadata {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        compression: CompressionMetadata {
            codec: "opus".to_string(),
            container_extension: defaults.extension.to_string(),
            sample_rate_hz: defaults.sample_rate_hz,
            channels: defaults.channels,
            bitrate: defaults.bitrate.to_string(),
        },
        source_audio,
        output_audio,
        metadata_artifacts,
    };

    let bundle_path = request.output_dir.join("bundle.json");
    let mut bundle_file = File::create(&bundle_path)
        .map_err(|err| format!("failed to create {}: {err}", bundle_path.display()))?;
    serde_json::to_writer_pretty(&mut bundle_file, &bundle)
        .map_err(|err| format!("failed to write bundle metadata: {err}"))?;
    bundle_file
        .write_all(b"\n")
        .map_err(|err| format!("failed to finalize bundle metadata: {err}"))?;

    Ok(CompressResult {
        bundle_dir: request.output_dir.clone(),
        audio_path: output_audio_path,
    })
}

pub fn compress_to_dir(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<CompressResult, String> {
    compress(&CompressRequest::new(input, output_dir))
}

fn is_supported_audio_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        &ext.to_ascii_lowercase()[..],
        "wav" | "mp3" | "m4a" | "aac" | "mp4" | "mov" | "ogg" | "flac"
    )
}

fn collect_audio_inputs(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(root)
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_audio_inputs(&path, out)?;
        } else if path.is_file() && is_supported_audio_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn bundle_dir_for_input(input_root: &Path, input: &Path, output_root: &Path) -> Result<PathBuf, String> {
    let relative = input.strip_prefix(input_root).map_err(|err| {
        format!(
            "failed to compute relative path for {} under {}: {err}",
            input.display(),
            input_root.display()
        )
    })?;
    let mut bundle_dir = output_root.join(relative);
    bundle_dir.set_extension("");
    Ok(bundle_dir)
}

pub fn compress_directory_to_dir(
    input_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<BatchCompressResult, String> {
    let input_root = input_root.as_ref();
    let output_root = output_root.as_ref();
    if !input_root.exists() {
        return Err(format!("input root does not exist: {}", input_root.display()));
    }
    if !input_root.is_dir() {
        return Err(format!("input root is not a directory: {}", input_root.display()));
    }
    if output_root.exists() {
        fs::remove_dir_all(output_root)
            .map_err(|err| format!("failed to remove {}: {err}", output_root.display()))?;
    }
    fs::create_dir_all(output_root)
        .map_err(|err| format!("failed to create {}: {err}", output_root.display()))?;

    let mut inputs = Vec::new();
    collect_audio_inputs(input_root, &mut inputs)?;
    inputs.sort();

    let mut items = Vec::with_capacity(inputs.len());
    for input in inputs {
        let bundle_dir = bundle_dir_for_input(input_root, &input, output_root)?;
        let result = compress_to_dir(&input, &bundle_dir)?;
        items.push(BatchItemResult {
            input,
            bundle_dir: result.bundle_dir,
            audio_path: result.audio_path,
        });
    }

    Ok(BatchCompressResult {
        output_root: output_root.to_path_buf(),
        item_count: items.len(),
        items,
    })
}

pub fn load_bundle_metadata(bundle_dir: impl AsRef<Path>) -> Result<BundleMetadata, String> {
    let path = bundle_dir.as_ref().join("bundle.json");
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{BundleMetadata, CompressionMetadata, MetadataArtifacts, OutputAudioMetadata, ProducerMetadata, SourceAudioMetadata, bundle_dir_for_input, is_supported_audio_file, load_bundle_metadata};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        std::env::temp_dir().join(format!("lifelog-audio-compression-{name}-{millis}"))
    }

    #[test]
    fn supports_common_audio_extensions_case_insensitively() {
        assert!(is_supported_audio_file(Path::new("clip.WAV")));
        assert!(is_supported_audio_file(Path::new("clip.mp4")));
        assert!(is_supported_audio_file(Path::new("clip.M4A")));
        assert!(!is_supported_audio_file(Path::new("clip.jpg")));
    }

    #[test]
    fn batch_bundle_dir_preserves_relative_structure() {
        let input_root = Path::new("/tmp/input");
        let input = Path::new("/tmp/input/NO NAME/TX00_MIC043_orig.wav");
        let output_root = Path::new("/tmp/output");
        let bundle_dir = bundle_dir_for_input(input_root, input, output_root).unwrap();
        assert_eq!(bundle_dir, Path::new("/tmp/output/NO NAME/TX00_MIC043_orig"));
    }

    #[test]
    fn load_bundle_metadata_reads_bundle_json() {
        let dir = temp_dir("bundle");
        fs::create_dir_all(&dir).unwrap();
        let bundle = BundleMetadata {
            format: "audio-bundle".to_string(),
            format_version: 1,
            bundle_id: "ab_test".to_string(),
            created_at_unix_ms: 1,
            producer: ProducerMetadata {
                name: "lifelog-audio-compression".to_string(),
                version: "0.1.0".to_string(),
            },
            compression: CompressionMetadata {
                codec: "opus".to_string(),
                container_extension: "ogg".to_string(),
                sample_rate_hz: 16_000,
                channels: 1,
                bitrate: "24k".to_string(),
            },
            source_audio: SourceAudioMetadata {
                source_relpath: "clip.wav".to_string(),
                file_size_bytes: 123,
                container_format: "wav".to_string(),
                audio_codec: "pcm_f32le".to_string(),
                duration_ms: 1000,
                sample_rate_hz: 48_000,
                channels: 1,
                channel_layout: Some("mono".to_string()),
                bits_per_sample: 32,
                creation_time: None,
            },
            output_audio: OutputAudioMetadata {
                audio_relpath: "audio.ogg".to_string(),
                file_size_bytes: 12,
                duration_ms: 1000,
                sample_rate_hz: 16_000,
                channels: 1,
            },
            metadata_artifacts: MetadataArtifacts {
                ffprobe_relpath: "source-ffprobe.json".to_string(),
                mdls_relpath: "source-mdls.txt".to_string(),
                xattrs_relpath: "source-xattrs.txt".to_string(),
            },
        };
        fs::write(
            dir.join("bundle.json"),
            serde_json::to_string_pretty(&bundle).unwrap(),
        )
        .unwrap();

        let loaded = load_bundle_metadata(&dir).unwrap();
        assert_eq!(loaded, bundle);
        fs::remove_dir_all(dir).unwrap();
    }
}
