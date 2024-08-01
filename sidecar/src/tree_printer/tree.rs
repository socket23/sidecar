use std::{fs, io, path::Path};

pub struct TreePrinter {}

impl TreePrinter {
    pub fn print(root: &Path) -> io::Result<()> {
        println!("{}", root.display());
        let (dirs, files) = TreePrinter::print_tree(root, "")?;
        println!("\n{} directories, {} files", dirs, files);
        Ok(())
    }

    fn print_tree(path: &Path, prefix: &str) -> io::Result<(usize, usize)> {
        const OTHER_CHILD: &str = "│   ";
        const OTHER_ENTRY: &str = "├── ";
        const FINAL_CHILD: &str = "    ";
        const FINAL_ENTRY: &str = "└── ";

        let mut dirs = 0;
        let mut files = 0;

        let entries = fs::read_dir(path)?.collect::<Result<Vec<_>, io::Error>>()?;
        let mut count = entries.len();

        for entry in entries {
            count -= 1;
            let connector = if count == 0 { FINAL_ENTRY } else { OTHER_ENTRY };
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();

            if path.is_dir() {
                println!("{}{}{}", prefix, connector, name);
                let new_prefix = format!(
                    "{}{}",
                    prefix,
                    if count == 0 { FINAL_CHILD } else { OTHER_CHILD }
                );
                let (sub_dirs, sub_files) = TreePrinter::print_tree(&path, &new_prefix)?;
                dirs += 1 + sub_dirs;
                files += sub_files;
            } else if path.is_symlink() {
                let target = fs::read_link(&path)?;
                println!(
                    "{}{}{} -> {}",
                    prefix,
                    connector,
                    name,
                    target.to_string_lossy()
                );
                files += 1;
            } else {
                println!("{}{}{}", prefix, connector, name);
                files += 1;
            }
        }

        Ok((dirs, files))
    }
}
