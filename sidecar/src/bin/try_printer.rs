use sidecar::{chunking::languages::TSLanguageParsing, repomap::tree_printer::TreePrinter};

fn main() {
    let filenames = vec!["test.ts".to_string()];

    let test_code = "let value: string | undefined | null = null;
        value = 'hello';
        value = undefined;"
        .to_string();

    let ts_parsing = TSLanguageParsing::init();

    let config = ts_parsing.for_file_path("test.ts").unwrap();

    let tree = config.get_tree_sitter_tree(test_code.as_bytes()).unwrap();

    let mut printer = TreePrinter::new(&tree, test_code).unwrap();

    let mut cursor = tree.walk();

    printer.walk_tree(&mut cursor);
}
