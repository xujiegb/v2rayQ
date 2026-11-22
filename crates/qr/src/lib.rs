#![forbid(unsafe_code)]
//! Bounded QR image decoding, independent of native graphics and client state.
use std::io::Cursor;

pub fn decode(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.len() > 8 * 1024 * 1024 { return Err("QR image exceeds 8 MiB".into()); }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format().map_err(|_| "Could not read QR image")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| "Use a PNG or JPEG image up to 4096 × 4096")?.to_luma8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 { return Err("Could not read QR image".into()); }
    let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(width as usize, height as usize, |x, y| image.get_pixel(x as u32, y as u32)[0]);
    let mut contents = Vec::new();
    for grid in prepared.detect_grids() {
        if let Ok((_, text)) = grid.decode() {
            let text = text.trim();
            if !text.is_empty() && !contents.iter().any(|saved| saved == text) { contents.push(text.to_owned()); }
        }
    }
    if contents.is_empty() { return Err("No supported QR code found".into()); }
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_images_fail_without_panicking() {
        assert!(decode(b"not an image").is_err());
        assert!(decode(&[]).is_err());
    }

    #[test]
    fn reads_generated_fixture_when_supplied() -> Result<(), Box<dyn std::error::Error>> {
        let Some(path) = std::env::var_os("V2RAYQ_QR_TEST_IMAGE") else { return Ok(()); };
        let contents = decode(&std::fs::read(path)?)?;
        assert_eq!(contents, ["trojan://secret@example.org:443#QR"]);
        Ok(())
    }
}
