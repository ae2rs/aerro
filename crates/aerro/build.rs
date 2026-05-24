fn main() {
    if std::env::var_os("CARGO_FEATURE_TONIC").is_some() {
        tonic_build::configure()
            .build_client(false)
            .build_server(false)
            .compile_protos(&["proto/aerro.v1.proto"], &["proto"])
            .expect("compile aerro.v1.proto");
    }
    println!("cargo:rerun-if-changed=proto/aerro.v1.proto");
}
