fn main() {
    cc::Build::new()
        .file("native/gaze.m")
        .flag("-fobjc-arc")
        .flag("-mmacosx-version-min=12.0")
        .compile("gaze_native");
    for framework in [
        "AppKit",
        "ApplicationServices",
        "CoreGraphics",
        "AVFoundation",
        "Vision",
        "CoreMedia",
        "CoreVideo",
        "ImageIO",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    println!("cargo:rustc-link-arg=-mmacosx-version-min=12.0");
    let clang = std::process::Command::new("xcrun")
        .args(["clang", "--print-resource-dir"])
        .output()
        .expect("Missing clang");
    assert!(clang.status.success());
    let resource = String::from_utf8(clang.stdout).unwrap();
    println!(
        "cargo:rustc-link-arg={}/lib/darwin/libclang_rt.osx.a",
        resource.trim()
    );
    println!("cargo:rerun-if-changed=native/gaze.m");
    println!("cargo:rerun-if-changed=native/guide.inc");
    println!("cargo:rerun-if-changed=native/learn.inc");
}
