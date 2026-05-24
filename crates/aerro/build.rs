fn main() {
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .compile_protos(&["proto/aerro.v1.proto"], &["proto"])
        .expect("compile aerro.v1.proto");
    println!("cargo:rerun-if-changed=proto/aerro.v1.proto");
}
