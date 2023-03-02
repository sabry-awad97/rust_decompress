use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

fn main() -> std::io::Result<()> {
    let path = Path::new("test.zip");
    let file = File::open(&path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let mut outfile = File::create(&outpath)?;
        std::io::copy(&mut file, &mut outfile)?;
    }

    Ok(())
}
