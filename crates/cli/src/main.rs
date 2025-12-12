use std::env;
use std::fs;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};

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
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <directory or .dcm file> <.tiff file>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);
    let dicom_paths = get_dicom_files(input_path)?;
    dicom2tiff::convert_dicom_files(dicom_paths, output_path)?;
    Ok(())
}