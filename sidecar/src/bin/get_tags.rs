use sidecar::chunking::languages::TSLanguageParsing;

fn main() {
    let ts_language_parsing = TSLanguageParsing::init();

    let test_file_path = "test.py";

    let lang = ts_language_parsing.detect_lang(&test_file_path);

    if lang.is_none() {
        println!("Language not detected");
        return;
    }

    println!("Language: {:?}", lang.unwrap());
}

