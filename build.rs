use std::io::Write;
use std::{
    env,
    ffi::OsStr,
    fs::{read_dir, read_to_string, File},
    path::Path,
};

fn main() {
    // Copy over the model files to where the binary gets generated at
    // copy_model_files();
    // This will run the migrations scripts for the sqlx
    let important_files_which_trigger_reindexing = [
        "src/indexes/file.rs",
        "src/indexes/schema.rs",
        "src/chunking/languages.rs",
        "src/semantic_search/schema.rs",
    ];
    let sql_schema_files = ["migrations"];
    let mut hasher = blake3::Hasher::new();
    for path in important_files_which_trigger_reindexing {
        hasher.update(read_to_string(path).unwrap().as_bytes());
        println!("cargo:rerun-if-changed={path}");
    }
    for path in sql_schema_files
        .iter()
        .flat_map(|dir| read_dir(dir).unwrap())
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            // if Some(OsStr::new("rs")) == path.extension() {
            //     Some(path)
            // } else {
            //     None
            // }
            Some(path)
        })
    {
        hasher.update(read_to_string(&path).unwrap().as_bytes());
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=src");
    let version_file = Path::new(&env::var("OUT_DIR").unwrap()).join("version_hash.rs");
    write!(
        File::create(version_file).unwrap(),
        r#"pub const BINARY_VERSION_HASH: &str = "{}";"#,
        hasher.finalize()
    )
    .unwrap();
}

// fn copy_model_files() {
//     // Where is the out dir located?
//     let current_directory = env::current_dir().unwrap();
//     // Now we want to deep copy over the src/models folder in the current
//     // directory where the build is running and paste it beside the binary
//     // or as a subfolder here
//     let src_models_dir = current_directory.join("src").join("models");
//     let out_dir_env = env::var("OUT_DIR").unwrap();
//     let output_directory = Path::new(&out_dir_env);
//     // We need to make sure this path exists
//     let out_models_dir = output_directory.join("models");
//     fs::create_dir_all(&out_models_dir).unwrap();
//     println!("We are over here at copy_model_files");
//     // Now we want to copy over the src/models folder to the out/models folder
//     // We can use the fs_extra crate to do this
//     fs_extra::dir::copy(
//         src_models_dir,
//         out_models_dir,
//         &fs_extra::dir::CopyOptions::new(),
//     )
//     .unwrap();
// }
