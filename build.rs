fn main() {
    gen_proto();
}

#[cfg(feature = "v3")]
fn gen_proto() {
    protoc_rust_grpc::run(protoc_rust_grpc::Args {
        out_dir: "src/v3",
        includes: &["proto"],
        input: &["proto/rpc.proto", "proto/auth.proto", "proto/kv.proto"],
        rust_protobuf: true,
        ..Default::default()
    })
    .expect("protoc-rust-grpc failed");
}

#[cfg(not(feature = "v3"))]
fn gen_proto() {}
