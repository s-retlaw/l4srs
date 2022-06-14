 
#pacman -S musl
#rustup target add x86_64-unknown-linux-musl --toolchain=nightly
## when building the binary for deployment
#cargo build --release --target x86_64-unknown-linux-musl

cargo +nightly build --release --target x86_64-unknown-linux-musl
#upx --brute  target/x86_64-unknown-linux-musl/release/l4srs

