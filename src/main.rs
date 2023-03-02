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

struct ZipExtractor<'a> {
    archive: zip::ZipArchive<&'a File>,
    output_dir: PathBuf,
    progress_bar: Option<ProgressBar>,
}

impl<'a> ZipExtractor<'a> {
    fn new(zip_file: &'a File, output_dir: PathBuf, progress: bool) -> Result<Self, ExtractError> {
        let archive = zip::ZipArchive::new(zip_file)?;
        let progress_bar = if progress {
            let pb = ProgressBar::new(archive.len() as u64);
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
        Ok(Self {
            archive,
            output_dir,
            progress_bar,
        })
    }

    fn extract(&mut self) -> Result<Vec<ExtractedFile>, ExtractError> {
        let extracted_files = self.get_extracted_files()?;
        self.write_extracted_files(&extracted_files)?;
        self.finish_progress_bar(&extracted_files)?;
        Ok(extracted_files)
    }

    fn get_extracted_files(&mut self) -> Result<Vec<ExtractedFile>, ExtractError> {
        let extracted_files = (0..self.archive.len())
            .filter_map(|i| {
                let file = self.archive.by_index(i).ok()?;
                let outpath = match file.enclosed_name() {
                    Some(path) => self.output_dir.join(path),
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

        Ok(extracted_files)
    }

    fn write_extracted_files(
        &mut self,
        extracted_files: &Vec<ExtractedFile>,
    ) -> Result<(), ExtractError> {
        for extracted_file in extracted_files {
            match extracted_file.kind {
                FileKind::Directory => {
                    let dir_path = &extracted_file.path;
                    if !dir_path.exists() {
                        fs::create_dir_all(dir_path)?;
                    }
                }
                FileKind::File { .. } => {
                    let outpath = &extracted_file.path;
                    let mut outfile = fs::File::create(outpath)?;
                    let mut reader = self.archive.by_index(extracted_file.index)?;
                    io::copy(&mut reader, &mut outfile)?;
                }
            }

            if let Some(pb) = &mut self.progress_bar {
                pb.inc(1);
            }
        }

        Ok(())
    }

    fn finish_progress_bar(
        &mut self,
        extracted_files: &Vec<ExtractedFile>,
    ) -> Result<(), ExtractError> {
        if let Some(pb) = &mut self.progress_bar {
            pb.finish_with_message(format!("Extracted {} files", extracted_files.len()));
        }

        Ok(())
    }
}

fn extract(opt: Opt) -> Result<(), ExtractError> {
    let output_dir = opt
        .output_dir
        .unwrap_or_else(|| PathBuf::from(".").join(opt.input.file_stem().unwrap()));
    let zip_file = File::open(opt.input)?;
    let mut extractor = ZipExtractor::new(&zip_file, output_dir, opt.progress)?;
    extractor.extract()?;
    Ok(())
}

fn main() {
    let opt = Opt::from_args();
    if let Err(err) = extract(opt) {
        eprintln!("Error: {:?}", err);
    }
}
