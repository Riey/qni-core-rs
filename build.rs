use protoc_rust::Customize;
use std::process;

fn main() {
    process::Command::new("git").args(
        &[
            "clone",
            "--depth 1",
            "https://github.com/Riey/qni-api-protos",
            "api",
        ]
    ).spawn().expect("git");

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
