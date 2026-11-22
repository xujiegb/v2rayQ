use std::{env, fs, io, path::Path};

pub fn generate(scope: &str, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let folder = root.join("locales").join(scope);
    println!("cargo:rerun-if-changed={}", folder.display());
    let mut files: Vec<_> = fs::read_dir(&folder)?.collect::<Result<_, _>>()?;
    files.sort_by_key(|entry| entry.file_name());
    let mut source = String::from("pub const LOCALES: &[(&str, &str)] = &[\n");
    let mut manifest = Vec::new();
    for entry in files {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let code = path
            .file_stem()
            .and_then(|part| part.to_str())
            .ok_or("Invalid language filename")?;
        if !code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err("Invalid language code".into());
        }
        let contents = fs::read_to_string(&path)?;
        let catalog: std::collections::BTreeMap<String, String> = serde_json::from_str(&contents)?;
        let name = catalog.get("_name").ok_or("Language file requires _name")?;
        manifest.push(serde_json::json!({ "code": code, "name": name }));
        source.push_str(&format!(
            "({code:?}, include_str!({:?})),\n",
            path.canonicalize()?
        ));
    }
    if !manifest.iter().any(|language| language["code"] == "en") {
        return Err(io::Error::other("English language file is required").into());
    }
    source.push_str("];\n");
    source.push_str(&format!(
        "pub const MANIFEST: &str = {:?};\n",
        serde_json::to_string(&manifest)?
    ));
    fs::write(Path::new(&env::var("OUT_DIR")?).join("locales.rs"), source)?;
    Ok(())
}
