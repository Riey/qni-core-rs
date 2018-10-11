use protoc_rust::Customize;

fn main() {

    protoc_rust::run(protoc_rust::Args {
        out_dir: "src/protos",
        input: &["api/qni-api.proto"],
        includes: &["api"],
        customize: Customize {
            ..Default::default()
        },
    }).expect("protoc");
}
