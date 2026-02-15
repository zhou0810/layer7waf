use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let go_dir = manifest_dir.join("go");

    // Check that Go is installed
    let go_version = Command::new("go")
        .arg("version")
        .output()
        .expect("Go is not installed or not in PATH. Install Go >= 1.22 to build coraza bridge.");
    assert!(
        go_version.status.success(),
        "Failed to run `go version`: {}",
        String::from_utf8_lossy(&go_version.stderr)
    );

    // Build the Go shared library
    let output = Command::new("go")
        .args(["build", "-buildmode=c-shared", "-o", "libcoraza_bridge.so", "."])
        .current_dir(&go_dir)
        .output()
        .expect("Failed to execute `go build`");

    if !output.status.success() {
        panic!(
            "Go build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Tell cargo where to find the shared library
    println!(
        "cargo:rustc-link-search=native={}",
        go_dir.display()
    );
    println!("cargo:rustc-link-lib=dylib=coraza_bridge");

    // Rebuild when Go source changes
    println!("cargo:rerun-if-changed=go/main.go");
}
