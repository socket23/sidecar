use sidecar::chunking::languages::TSLanguageParsing;

const PYTHON_TS_QUERY: &str = "(class_definition
    name: (identifier) @name.definition.class) @definition.class
  
  (function_definition
    name: (identifier) @name.definition.function) @definition.function
  
  (call
    function: [
        (identifier) @name.reference.call
        (attribute
          attribute: (identifier) @name.reference.call)
    ]) @reference.call
  ";

struct Tag {
    name: String,
    kind: String,
}

pub fn get_tags_raw(file_path: &str) -> Option<Vec<Tag>> {
    let lang = self.detect_lang(file_path);
    let ts_query = self.for_lang(lang.unwrap());
}

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

