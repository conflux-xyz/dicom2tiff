use std::fs;
use std::io::{self, BufReader, Read, Seek};
use std::path::{Path, PathBuf};

use clap::Parser;
use tempfile::NamedTempFile;
use zip::ZipArchive;

/// Convert DICOM files to TIFF format
#[derive(Parser)]
#[command(name = "dicom2tiff")]
#[command(version, about, long_about = None)]
struct Args {
    /// Input path (directory, .dcm file, or .zip file). When using --single, this must be a .dcm file.
    input: PathBuf,

    /// Output .tiff file
    output: PathBuf,

    /// Process only the specified file (do not scan parent directory)
    #[arg(short, long)]
    single: bool,
}

fn is_dicom_file(path: &Path) -> bool {
    fs::File::open(path)
        .and_then(|mut f| {
            f.seek(io::SeekFrom::Start(128))?;
            let mut buf = [0u8; 4];
            f.read_exact(&mut buf)?;
            Ok(buf)
        })
        .map(|buf| buf == *b"DICM")
        .unwrap_or(false)
}

fn is_dicom_data<R: Read + Seek>(reader: &mut R) -> io::Result<bool> {
    reader.seek(io::SeekFrom::Start(128))?;
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    reader.rewind()?;
    Ok(buf == *b"DICM")
}

fn is_zip_file(path: &Path) -> bool {
    if let Ok(mut file) = fs::File::open(path) {
        let mut signature = [0u8; 4];
        if file.read_exact(&mut signature).is_ok() {
            return &signature == b"PK\x03\x04";
        }
    }
    false
}

fn get_dicom_files_from_zip(
    zip_path: &Path,
) -> Result<Vec<NamedTempFile>, Box<dyn std::error::Error>> {
    let file = fs::File::open(zip_path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    let mut dicom_files = Vec::new();

    for i in 0..archive.len() {
        let mut zip_file = archive.by_index(i)?;

        // Skip directories
        if zip_file.is_dir() {
            continue;
        }

        // Create a temporary file (will be auto-deleted when dropped)
        let mut temp_file = NamedTempFile::new()?;

        // Copy data from ZIP to temp file
        io::copy(&mut zip_file, &mut temp_file)?;

        // Seek back to start and check if it's a DICOM file
        temp_file.rewind()?;
        if is_dicom_data(&mut temp_file).unwrap_or(false) {
            // Seek back to the beginning for processing
            dicom_files.push(temp_file);
        }
        // If not a DICOM file, temp_file is dropped and auto-deleted
    }

    Ok(dicom_files)
}

fn get_dicom_files(path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let dir = if path.is_file() {
        path.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No parent directory"))?
    } else if path.is_dir() {
        path
    } else {
        return Err("Path is not a file or directory".into());
    };

    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_dicom_file(&path) {
            paths.push(path);
        }
    }

    Ok(paths)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input_path = &args.input;
    let output = fs::File::create(&args.output)?;

    if args.single {
        // Single file mode: only process the specified file
        if !input_path.is_file() {
            eprintln!("Error: --single requires a file path, not a directory");
            std::process::exit(1);
        }
        if !is_dicom_file(input_path) {
            eprintln!("Error: {} is not a valid DICOM file", input_path.display());
            std::process::exit(1);
        }
        let file = fs::File::open(input_path)?;
        let dicom_sources = vec![BufReader::new(file)];
        dicom2tiff::convert_dicom_sources(dicom_sources, output)?;
    // Check if the input is a ZIP file
    } else if input_path.is_file() && is_zip_file(input_path) {
        let dicom_files = get_dicom_files_from_zip(input_path)?;
        let dicom_sources: Vec<BufReader<_>> =
            dicom_files.into_iter().map(BufReader::new).collect();
        dicom2tiff::convert_dicom_sources(dicom_sources, output)?;
    } else {
        let dicom_paths = get_dicom_files(input_path)?;
        let dicom_sources: Vec<BufReader<_>> = dicom_paths
            .into_iter()
            .map(fs::File::open)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(BufReader::new)
            .collect();
        dicom2tiff::convert_dicom_sources(dicom_sources, output)?;
    }

    Ok(())
}
