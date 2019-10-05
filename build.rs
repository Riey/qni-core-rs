use protoc_rust::Customize;
use std::path;

fn main() {
    if path::Path::new("api/qni-api.proto").exists() {
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
}
