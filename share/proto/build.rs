fn main() {
    prost_build::Config::new()
        .type_attribute(".", "#[allow(non_camel_case_types)]")
        .compile_protos(&["protos/message.proto"], &["protos/"])
        .unwrap();
}
