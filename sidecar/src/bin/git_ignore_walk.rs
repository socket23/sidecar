#[tokio::main]
async fn main() {
    use ignore::Walk;

    for result in Walk::new("/Users/skcd/scratch/ide") {
        // Each item yielded by the iterator is either a directory entry or an
        // error, so either print the path or the error.
        match result {
            Ok(entry) => println!("{}", entry.path().display()),
            Err(err) => println!("ERROR: {}", err),
        }
    }
}
