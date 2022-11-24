#!/bin/zsh
# Apple universal binary build script
cargo build --release --target=aarch64-apple-darwin
cargo build --release --target=x86_64-apple-darwin
lipo -create target/aarch64-apple-darwin/release/trust-80 target/x86_64-apple-darwin/release/trust-80 -output ./trust-80
zip -9 teletype_mac_os.zip trust-80