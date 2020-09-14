function cargosan(){
  SANITIZER=$1
  shift
  RUSTFLAGS="-Z sanitizer=$SANITIZER" \
    cargo test \
    --target=x86_64-apple-darwin --target-dir=target/"$SANITIZER" \
    -- --nocapture "$@"
}
function testall(){
  cargo test -- --nocapture "$@"
  cargo test --release -- --nocapture "$@"
  cargosan thread "$@"
  cargosan leak "$@"
  cargosan address "$@"
}
