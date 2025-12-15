# dicom2tiff

Convert DICOM whole-slide imaging (WSI) files to multi-resolution pyramidal TIFF format.

This tool converts DICOM WSI files into BigTIFF format compatible with Aperio SVS viewers and analysis software. It preserves pyramid levels, resolution metadata, and color profiles while supporting various photometric interpretations and compression methods.

## Try it in your browser

Try out the in-browser WASM conversion at **[https://github.conflux.xyz/dicom2tiff/](https://github.conflux.xyz/dicom2tiff/)**. All conversions happen locally in your browser with no server uploads required.

## Features

- Convert DICOM WSI files to pyramidal TIFF format
- Support for multiple input formats:
  - Individual DICOM files (`.dcm`)
  - Directories containing DICOM files
  - ZIP archives with DICOM files
- Preserves pyramid levels and resolution metadata (MPP)
- Handles various photometric interpretations:
  - MONOCHROME1, MONOCHROME2
  - RGB
  - YBR_FULL, YBR_FULL_422, YBR_ICT
- Supports JPEG and JPEG2000 compression
- ICC profile preservation
- Available as CLI tool, Rust library, and WebAssembly module

## Installation

### CLI Tool

Download pre-built binaries from the [releases page](https://github.com/conflux-xyz/dicom2tiff/releases). The binary is named `dicom2tiff` and is available for multiple platforms including Linux, macOS, and Windows.

Alternatively, install via cargo:

```bash
cargo install --path crates/cli
```

Or build from source:

```bash
git clone https://github.com/conflux-xyz/dicom2tiff.git
cd dicom2tiff
cargo build --release -p dicom2tiff-cli
```

The binary will be available at `target/release/dicom2tiff-cli`.

### Library

Add to your `Cargo.toml`:

```toml
[dependencies]
dicom2tiff = { git = "https://github.com/conflux-xyz/dicom2tiff.git" }
```

### WebAssembly

Build the WASM package:

```bash
make build-wasm
```

The package will be generated in `crates/wasm/pkg/` as `@conflux-xyz/dicom2tiff`.

## Usage

### CLI

Convert a DICOM file or directory:

```bash
dicom2tiff-cli /path/to/dicom/file-or-directory output.tiff
```

Convert from a ZIP archive:

```bash
dicom2tiff-cli archive.zip output.tiff
```

### Rust Library

```rust
use std::fs::File;
use std::io::BufReader;
use dicom2tiff::convert_dicom_sources;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dicom_files = vec![
        BufReader::new(File::open("image1.dcm")?),
        BufReader::new(File::open("image2.dcm")?),
    ];

    let output = File::create("output.tiff")?;
    convert_dicom_sources(dicom_files, output)?;

    Ok(())
}
```

### WebAssembly

See the [web example](examples/web) for a complete implementation which (as scalably as possible) converts using
Web Workers and OPFS file handles.

## Development

### Prerequisites

- Rust 1.80+ (2024 edition)
- For WASM: [wasm-pack](https://rustwasm.github.io/wasm-pack/)

### Building

Build all components:

```bash
cargo build --all
```

Build CLI only:

```bash
make build-cli
```

Build WASM module:

```bash
make build-wasm
```

### Code Quality

Run clippy:

```bash
make clippy
```

Format code:

```bash
make format
```

### Cross-compilation

The project supports cross-compilation for multiple platforms via GitHub Actions. See [.github/workflows/draft-pre-release.yml](.github/workflows/draft-pre-release.yml) for supported targets.

## Architecture

The project is organized as a Cargo workspace with three crates:

- **`crates/core`**: Core conversion library with DICOM parsing and TIFF generation
- **`crates/cli`**: Command-line interface with support for files, directories, and ZIP archives
- **`crates/wasm`**: WebAssembly bindings for browser usage

## Limitations

- Only includes pyramid levels in the output TIFF; associated images (label, macro) are not currently included
- Currently does not support sparsely tiled DICOM images (TILED_SPARSE)
- Requires DICOM files to have specific ImageType values for pyramid level detection
- JPEG tables extraction for shared JPEG compression is not yet implemented

## Contributing

Contributions are welcome. Please ensure code passes clippy checks and follows the existing style.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

Built with the [DICOM-rs](https://github.com/Enet4/dicom-rs) library for DICOM parsing and the [image-tiff](https://github.com/image-rs/image-tiff) library for TIFF encoding.
