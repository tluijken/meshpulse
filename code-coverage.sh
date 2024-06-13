CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test --manifest-path=./meshpulse/Cargo.toml
grcov . --binary-path ./meshpulse/target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o ./meshpulse/target/coverage/html
rm ./meshpulse/*.profraw
# open the coverage report in the browser
xdg-open ./meshpulse/target/coverage/html/index.html
