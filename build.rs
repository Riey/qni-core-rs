use protoc_rust::Customize;
use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);
    let api_dir = out_dir.join("api");
    let proto_dir = api_dir.join("qni-api.proto");
    let api_repo = git2::Repository::clone("https://github.com/Riey/qni-api-protos", &api_dir)
        .expect("clone api repo");

    api_repo.set_head_detached(git2::Oid::from_str("7361b5c75c66692a8332a3b1ed54aaf31ca189af").unwrap()).unwrap();

        protoc_rust::run(protoc_rust::Args {
            out_dir: "src/protos",
            input: &[proto_dir.to_str().unwrap()],
            includes: &[api_dir.to_str().unwrap()],
            customize: Customize {
                ..Default::default()
            },
        })
            .expect("protoc");
}
