#![forbid(unsafe_code)]

// Optional terminal UI. No async SQL runtime or Turso dependency.
fn main() -> std::io::Result<()> {
    v2rayQ::cli::run()
}
