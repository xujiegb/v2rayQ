#[path = "../../locales/embed.rs"]
mod embed;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::var("CARGO_MANIFEST_DIR")?;
    embed::generate(
        "server",
        std::path::Path::new(&folder).join("../..").as_path(),
    )
}
