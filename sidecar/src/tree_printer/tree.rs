use std::{fs, io, path::Path};

pub struct TreePrinter {}

impl TreePrinter {
    pub fn new() -> Self {
        TreePrinter {}
    }

    pub fn print(root: &Path) -> io::Result<()> {
        println!("{}", root.display());
        let (tree, dirs, files) = TreePrinter::to_string(root)?;
        println!("{}", tree);
        println!("\n{} directories, {} files", dirs, files);
        Ok(())
    }

    pub fn to_string(root: &Path) -> io::Result<(String, usize, usize)> {
        let mut output = String::new();
        let (dirs, files) = TreePrinter::build_tree_string(root, "", &mut output)?;

        Ok((output, dirs, files))
    }

    fn build_tree_string(
        path: &Path,
        prefix: &str,
        output: &mut String,
    ) -> io::Result<(usize, usize)> {
        const OTHER_CHILD: &str = "│   ";
        const OTHER_ENTRY: &str = "├── ";
        const FINAL_CHILD: &str = "    ";
        const FINAL_ENTRY: &str = "└── ";

        let mut dirs = 0;
        let mut files = 0;

        let entries = fs::read_dir(path)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    let file_name = path.file_name().and_then(|n| n.to_str());

                    if path.is_dir() && file_name.map_or(false, |name| name.starts_with(".")) {
                        None
                    } else {
                        Some(e)
                    }
                })
            })
            .collect::<Vec<_>>();

        let mut count = entries.len();

        for entry in entries {
            count -= 1;
            let connector = if count == 0 { FINAL_ENTRY } else { OTHER_ENTRY };
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();

            if path.is_dir() {
                output.push_str(&format!("{}{}{}\n", prefix, connector, name));
                let new_prefix = format!(
                    "{}{}",
                    prefix,
                    if count == 0 { FINAL_CHILD } else { OTHER_CHILD }
                );
                let (sub_dirs, sub_files) =
                    TreePrinter::build_tree_string(&path, &new_prefix, output)?;
                dirs += 1 + sub_dirs;
                files += sub_files;
            } else if path.is_symlink() {
                let target = fs::read_link(&path)?;
                output.push_str(&format!(
                    "{}{}{} -> {}\n",
                    prefix,
                    connector,
                    name,
                    target.to_string_lossy()
                ));
                files += 1;
            } else {
                output.push_str(&format!("{}{}{}\n", prefix, connector, name));
                files += 1;
            }
        }

        Ok((dirs, files))
    }
}
