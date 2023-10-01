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
- you can use the following command to do the migrations etc:
- cargo sqlx prepare --database-url=sqlite://codestory.db