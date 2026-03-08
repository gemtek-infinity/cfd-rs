#![forbid(unsafe_code)]

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

fn main() {
    eprintln!("cloudflared Rust rewrite workspace scaffold: no runtime behavior is implemented yet");
}
