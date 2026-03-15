// Compile Cap'n Proto schemas from the frozen Go baseline into Rust bindings.
//
// The schemas in baseline-2026.2.0/tunnelrpc/proto/ are the wire-format truth
// for registration RPC and stream framing. This build step generates typed
// Rust readers and builders that match the exact byte layout the Cloudflare
// edge expects.

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
