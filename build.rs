use protoc_rust::Customize;
use std::path;

fn main() {
    if path::Path::new("api/qni-api.proto").exists() {
        protoc_rust::Codegen::new()
            .out_dir("src/protos")
            .input("api/qni-api.proto")
            .include("api")
            .customize(Customize {
                serde_derive: Some(true),
                ..Default::default()
            })
            .run()
            .expect("protoc");
    }
}
