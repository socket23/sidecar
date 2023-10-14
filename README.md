# sidecar

## Why sidecar?

We need an additional binary which we can offload the heavy lifting to and use that instead of just using the normal binary as its slow as shit


## onnx runtime and wtf
- We need a onnx runtime which we are checking into the repository to make sure that we can enable ort
 to work as we want it to
- we could check in this binary into the repo and have it built along with the editor to keep both of them intact and make sure that we are not regressing anywhere
- So for the binary to work we need the following things to be present at the
 right location: models folder and also the libonnxruntime file depending on the platform
- once we can get these things sorted we are in a good position to run and package
the binary 

## We are not going to parallelize anything, we are proud of being lazy
- fix the speed etc when we hit issues with it

## How to install sqlx and migrations
- for sqlx install it using `cargo install sqlx`
- and then for the migrations which are present in the ./migrations folder where we have Cargo.toml, we need to add migrations using `sqlx migrate add {blah}`
- after making the edits to the file remember to run this: cargo `sqlx migrate run --database-url=sqlite://codestory.db`
- you can use the following command to do the migrations etc:
- cargo sqlx prepare --database-url=sqlite://codestory.db

## Qdrant binary and where to download
- To download the binaries, you can visit this: https://github.com/qdrant/qdrant/releases/tag/v1.2.0
- download from here and update the binary where required


## What keys are important here?
- We need to have a single key which can map back to the semantic algorithm we are using, cause tantivy is sensitive to changes
 in the keys
- Then we need a key to identify the file using the file path (we can use that to lookup everything about a file and update things)
- Lastly we also need a key which can be used to track the commit hash associated with the repo when we are indexing
- And another key which is the hash of the file content in the file, this will be useful to make sure that we can see if things have changed or not and decide accordingly

Database structure:
file_cache: file_path, repo_ref, tantivy_cache_key, file_hash
chunk_cache: file_path, repo_ref, chunk_hash, line_start, line_end, tantivy_cache_key


## What are the important files where we need to rebuild the database again?
- semantic_search/schema.rs is one of them

## Where do we get the stopwords from?
- https://github.com/aneesha/RAKE/blob/master/SmartStoplist.txt here's where we are getting the list from

## How to start the binary?
- I am using this command as we also need to provide the qdrant binary
`./target/debug/webserver --qdrant-binary-directory /Users/skcd/scratch/sidecar/qdrant --dylib-directory /Users/skcd/scratch/sidecar/onnxruntime/ --model-dir /Users/skcd/scratch/sidecar/models/all-MiniLM-L6-v2/ --qdrant-url http://127.0.0.1:6334`
- For large repos we have to set the ulimit on the shell where the binary will be running manually by using ulimit -n 16535 or similar

## Gotcha's
- If qdrant explodes while indexing, you might need to increase the ulimit on your machine. You can do that by running: sudo ulimit -n 16535