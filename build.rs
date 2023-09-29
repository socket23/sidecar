use std::{env, path::Path};

fn main() {
    println!("Are we building???");
    let out_dir_env = env::var("OUT_DIR").unwrap();
    let output_directory = Path::new(&out_dir_env);
    println!("{:?}", output_directory);
    println!("{:?}", env::current_dir());
    // Copy over the model files to where the binary gets generated at
    // copy_model_files();
    println!("cargo:rerun-if-changed=src");
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
