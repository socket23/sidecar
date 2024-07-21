use sidecar::{chunking::languages::TSLanguageParsing, repomap::tree_printer::TreeContext};

fn main() {
    let printer = TreeContext::new(
        "test.ts".to_string(),
        "let value: string | undefined | null = null;
        value = 'hello';
        value = undefined;"
            .to_string(),
        &TSLanguageParsing::init(),
    );
}
