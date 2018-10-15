use protoc_rust::Customize;
use rustc_version::{version_meta, Channel};

fn main() {

    match version_meta().unwrap().channel {
        Channel::Nightly => {
            println!("cargo:rustc-cfg=NIGHTLY");
        }
        _ => {}
    };

    protoc_rust::run(protoc_rust::Args {
        out_dir: "src/protos",
        input: &["api/qni-api.proto"],
        includes: &["api"],
        customize: Customize {
            ..Default::default()
        },
    })
    .expect("protoc");
}
