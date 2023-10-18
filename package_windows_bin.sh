cargo build --bin webserver --release

pathsToZip="onnxruntime/ qdrant/ target/release/webserver models/"

# Destination of the zip file
zipFileDestination="sidecar.7z"

# Use 7z command to create the archive
7z a -t7z $zipFileDestination $pathsToZip