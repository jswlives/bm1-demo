fn main() {
    let out_dir = std::path::PathBuf::from("../protos_build");
    prost_build::Config::new()
        .type_attribute(".", "#[allow(non_camel_case_types)]")
        .out_dir(&out_dir)
        .compile_protos(&["protos/message.proto", "protos/model.proto"], &["protos/"])
        .unwrap();
}
