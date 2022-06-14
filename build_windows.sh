# rustup target add x86_64-pc-windows-gnu
# rustup toolchain install stable-x86_64-pc-windows-gnu
# cargo build --target x86_64-pc-windows-gnu

cargo +nightly build --release --target x86_64-pc-windows-gnu
#upx --brute target/x86_64-pc-windows-gnu/release/l4srs.exe

