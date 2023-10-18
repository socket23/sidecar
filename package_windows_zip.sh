cargo build --target=x86_64-pc-windows-gnu --verbose --release
zip -r sidecar onnxruntime/ qdrant/ target/x86_64-pc-windows-gnu/release models/