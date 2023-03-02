use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "unzip", about = "Extracts files from a zip archive")]
struct Opt {
    /// The zip file to extract
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// The directory to extract the files to
    #[structopt(parse(from_os_str))]
    output_dir: Option<PathBuf>,

    /// Show a progress bar
    #[structopt(short, long)]
    progress: bool,
}

#[derive(Debug)]
enum ExtractError {
    IoError(io::Error),
    ZipError(zip::result::ZipError),
}

impl From<io::Error> for ExtractError {
    fn from(err: io::Error) -> Self {
        ExtractError::IoError(err)
    }
}

impl From<zip::result::ZipError> for ExtractError {
    fn from(err: zip::result::ZipError) -> Self {
        ExtractError::ZipError(err)
    }
}

#[derive(Debug)]
enum FileKind {
    Directory,
    File { size: u64 },
}

#[derive(Debug)]
struct ExtractedFile {
    path: PathBuf,
    kind: FileKind,
    index: usize,
}

fn extract_files(
    zip_file: &File,
    output_dir: &PathBuf,
    progress: bool,
) -> Result<(), ExtractError> {
    let mut archive = zip::ZipArchive::new(zip_file)?;

    let total_files = archive.len();
    let progress_bar = if progress {
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap(),
        );
        pb.set_message("Extracting files...");
        Some(pb)
    } else {
        None
    };

    let extracted_files = (0..total_files)
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let outpath = match file.enclosed_name() {
                Some(path) => output_dir.join(path),
                None => return None,
            };

            let kind = if (*file.name()).ends_with('/') {
                FileKind::Directory
            } else {
                FileKind::File { size: file.size() }
            };

            Some(ExtractedFile {
                path: outpath,
                kind,
                index: i,
            })
        })
        .collect::<Vec<_>>();

    if let Some(pb) = &progress_bar {
        pb.finish_with_message(format!("Extracted {} files", total_files));
    }

    for extracted_file in extracted_files {
        match extracted_file.kind {
            FileKind::Directory => {
                let dir_path = extracted_file.path;
                fs::create_dir_all(&dir_path)?;
            }
            FileKind::File { size } => {
                let file_path = extracted_file.path;
                let parent_dir = file_path.parent().unwrap();
                fs::create_dir_all(&parent_dir)?;

                let mut file = fs::File::create(&file_path)?;
                let mut zip_file = archive.by_index(extracted_file.index)?;
                io::copy(&mut zip_file, &mut file)?;

                if let Some(pb) = &progress_bar {
                    pb.inc(1);
                    pb.set_message(format!("Extracted {:?}", file_path));
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), ExtractError> {
    let opt = Opt::from_args();

    let output_dir = opt.output_dir.unwrap_or_else(|| {
        let input_path = &opt.input;
        let output_path = input_path.with_extension("");
        output_path
    });

    let zip_file = File::open(opt.input)?;

    extract_files(&zip_file, &output_dir, opt.progress)?;

    Ok(())
}
