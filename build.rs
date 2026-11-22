#[path = "locales/embed.rs"]
mod embed;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    embed::generate(
        "client",
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR")?),
    )
}
