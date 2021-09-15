use std::process::Command;
use std::{env, fs};
use std::ffi::{OsStr};

#[must_use]
pub fn build(input_dir: &str) -> bool {
    println!("cargo:rerun-if-changed={}", input_dir);
    let paths = match fs::read_dir(input_dir) {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("Missing flatc input directory '{}': {}", input_dir, e);
            return false;
        }
    };
    let dest_path = match env::var_os("OUT_DIR") {
        Some(out_dir) => out_dir,
        None => {
            eprintln!("Missing environment variable 'OUT_DIR'");
            return false;
        }
    };
    let mut success = true;
    let mut command = Command::new("flatc");
    command.args(&[OsStr::new("--rust"), OsStr::new("-o"), &dest_path]);
    for path in paths {
        let path = match path {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Error processing input directory: {}", e);
                success = false;
                continue;
            }
        };
        let path = path.path();
        let path_str = match path.to_str() {
            Some(path_str) => path_str,
            None => {
                eprintln!("Error converting path to string '{:?}'", path);
                success = false;
                continue;
            }
        };
        if path.extension() != Some(&OsStr::new("fbs")) {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path_str);
        command.arg(path.as_os_str());
    }
    let mut child = command.spawn().unwrap();
    let status = child.wait();
    let status = match status {
        Ok(status) => status,
        Err(e) => {
            eprintln!("Failed to wait for flatc: {}", e);
            return false;
        }
    };
    if !status.success() {
        eprintln!("flatc failed.");
        return false;
    }
    success
}

