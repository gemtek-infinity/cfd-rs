pub(super) fn render_help(program_name: &str) -> String {
    let mut text = String::new();
    text.push_str(&format!("{program_name} {}\n", env!("CARGO_PKG_VERSION")));
    text.push_str(
        "Linux production-alpha QUIC tunnel core with wire/protocol boundary, Pingora proxy seam, and \
         narrow operability reporting\n\n",
    );
    text.push_str("Usage:\n");
    text.push_str("  cloudflared [--config FILEPATH] validate\n");
    text.push_str("  cloudflared [--config FILEPATH] run\n");
    text.push_str("  cloudflared help\n");
    text.push_str("  cloudflared version\n\n");
    text.push_str("Admitted commands:\n");
    text.push_str(
        "  validate  Resolve config, load YAML, normalize ingress, and report startup readiness.\n",
    );
    text.push_str(
        "  run       Enter the runtime-owned QUIC transport core with wire/protocol \
         boundary\n\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20and Pingora proxy \
         seam.\n\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20Emits narrow lifecycle, readiness, and \
         failure visibility for the\n\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20admitted alpha role. \
         The admitted origin path is http_status \
         only.\n\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20Broader origin support and general proxy \
         completeness remain later\n\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20slices.\n",
    );
    text.push_str("  version   Print the workspace version.\n");
    text.push_str("  help      Print this help text.\n\n");
    text.push_str("Admitted flags and defaults:\n");
    text.push_str("  --config FILEPATH  Use an explicit YAML config path.\n");
    text.push_str(
        "  default discovery  Search ~/.cloudflared, ~/.cloudflare-warp, ~/cloudflare-warp, \
         /etc/cloudflared, /usr/local/etc/cloudflared.\n",
    );
    text.push_str(
        "  default create     If no config exists, write /usr/local/etc/cloudflared/config.yml with \
         logDirectory: /var/log/cloudflared.\n\n",
    );
    text.push_str("Admitted environment:\n");
    text.push_str("  HOME  Expands the leading ~ in default config search directories.\n\n");
    text.push_str("Admitted operability surface:\n");
    text.push_str(
        "  run output  Reports runtime lifecycle, owner-scoped transport/protocol/proxy \
         state,\n\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20narrow readiness, and localized failure \
         visibility for the admitted path.\n\n",
    );
    text.push_str("Deferred beyond current phase:\n");
    text.push_str(
        "  Broader origin support, registration RPC, incoming stream handling,\n\x20\x20certificate/key \
         container handling beyond the active path, packaging, and deployment tooling\n",
    );
    text
}
