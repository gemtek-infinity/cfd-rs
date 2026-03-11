use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

use rmcp::{ServiceExt, transport::stdio};

mod cogload;
mod context;
mod fs;
mod log;
mod profile;
mod repo;
mod search;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::init();

    let repo_root = std::env::current_dir()?;
    let repo_root_canon = std::fs::canonicalize(&repo_root)?;

    let server = server::CfdRsMemory::new(repo_root, repo_root_canon)
        .serve(stdio())
        .await?;

    server.waiting().await?;
    Ok(())
}
