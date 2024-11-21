#[tokio::main]
async fn main() {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(
        "Hello World\nThis is the second line.\nThis is the third.\nMoar and more",
        "Hallo Welt\nThis is the second line.\nThis is life.\nMoar and more",
    );

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        print!("{}{}", sign, change);
    }
}
