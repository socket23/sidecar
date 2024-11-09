cargo build --bin webserver --release

pathsToZip="target/release/webserver.exe"

# Destination of the zip file
zipFileDestination="sidecar.zip"

# Use 7z command to create the archive
7z a -tzip $zipFileDestination $pathsToZip