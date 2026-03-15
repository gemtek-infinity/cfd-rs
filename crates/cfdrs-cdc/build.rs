fn main() {
    let baseline_proto = "../../baseline-2026.2.0/tunnelrpc/proto";

    capnpc::CompilerCommand::new()
        .src_prefix(baseline_proto)
        .import_path(baseline_proto)
        .file(format!("{baseline_proto}/tunnelrpc.capnp"))
        .file(format!("{baseline_proto}/quic_metadata_protocol.capnp"))
        .run()
        .expect("capnpc schema compilation failed");
}
