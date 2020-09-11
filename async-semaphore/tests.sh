function cargosan(){
  SANITIZER=$1
  shift
  RUSTFLAGS="-Z sanitizer=$SANITIZER" \
    cargo test --quiet \
    --target=x86_64-apple-darwin --target-dir=target/"$SANITIZER" \
    -- --nocapture "$@"
}
cargo test --quiet -- --nocapture "$@"
cargo test --quiet --release -- --nocapture "$@"
cargosan thread "$@"
cargosan leak "$@"
cargosan address "$@"