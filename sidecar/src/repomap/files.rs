use std::{
    io,
    path::{Path, PathBuf},
};

pub trait FileSystem {
    fn get_files(&self, dir: &Path) -> Result<Vec<PathBuf>, io::Error>;
}

struct SimpleFileSystem;

impl FileSystem for SimpleFileSystem {
    fn get_files(&self, dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
        let mut files = Vec::new();
        self.get_files_recursive(dir, &mut files)?;
        Ok(files)
    }
}

impl SimpleFileSystem {
    fn get_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                self.get_files_recursive(&path, files)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_files_in_repomap() {
        let fs = SimpleFileSystem;
        let dir = Path::new("src/");

        match fs.get_files(dir) {
            Ok(files) => {
                println!("Files in /src/");
                for file in files {
                    println!("{}", file.display());
                }
            }
            Err(e) => println!("Error reading directory: {}", e),
        }
    }
}
