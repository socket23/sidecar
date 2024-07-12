use sidecar::chunking::languages::TSLanguageParsing;

fn main() {
    let ts_language_parsing = TSLanguageParsing::init();
    let source_code = r#"
    fn main() {
        println!("Hello, world!");
    }
    "#;

    if let Some(_) = ts_language_parsing.for_lang("python") {
        let chunks = ts_language_parsing.chunk_file(
            "example.rs",
            source_code,
            Some("rs"),
            Some("rust"),
        );
        for chunk in chunks {
            println!("{:?}", chunk);
        }
    } else {
        println!("Language configuration not found for 'rust'");
    }
}