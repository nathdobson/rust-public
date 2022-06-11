fn main() {
    let mut success = true;
    success &= flatc_build::build("schema/");
    if !success {
        std::process::exit(1);
    }
}
