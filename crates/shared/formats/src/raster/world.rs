//! World files (docs/adr/0204 §1, §6): six numbers, a line each, A, D, B, E,
//! C, F, where x = A·i + B·j + C and y = D·i + E·j + F for the centre of the
//! pixel in column i and row j (0, 0 the upper left). KentOS's affine is to
//! pixel corners, so C and F move half a pixel either way.

use super::RasterError;

/// The world file's extensions for an image's: `.tif` takes `.tfw` (or
/// `.tifw`, `.wld`), `.png` `.pgw` and so on, as GDAL looks for them.
pub fn extensions(image_ext: &str) -> Vec<String> {
    let e = image_ext.trim_start_matches('.').to_ascii_lowercase();
    let mut out = Vec::new();
    if e.len() >= 3 {
        let b = e.as_bytes();
        out.push(format!("{}{}w", b[0] as char, b[e.len() - 1] as char));
    }
    out.push(format!("{e}w"));
    out.push("wld".to_owned());
    out
}

/// The extension a world file is written with for an image's.
pub fn extension_for(image_ext: &str) -> String {
    extensions(image_ext)
        .into_iter()
        .next()
        .unwrap_or_else(|| "wld".to_owned())
}

/// The affine `[x₀, a, b, y₀, c, d]` (pixel corners) a world file's text gives.
pub fn read(text: &str) -> Result<[f64; 6], RasterError> {
    let mut n = [0.0f64; 6];
    let mut k = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if k == 6 {
            break;
        }
        // Some writers use a decimal comma; a thousands separator never appears here.
        let v: f64 = t.replace(',', ".").parse().map_err(|_| {
            RasterError::new(format!(
                "Dünya dosyasının {}. satırı sayı değil: “{t}”.",
                k + 1
            ))
        })?;
        if !v.is_finite() {
            return Err(RasterError::new(format!(
                "Dünya dosyasının {}. satırı sonlu değil.",
                k + 1
            )));
        }
        n[k] = v;
        k += 1;
    }
    if k < 6 {
        return Err(RasterError::new("Dünya dosyası altı sayı içermiyor."));
    }
    let [a, d, b, e, c, f] = n;
    if a * e - b * d == 0.0 {
        return Err(RasterError::new("Dünya dosyasının dönüşümü tersinmiyor."));
    }
    Ok([c - 0.5 * a - 0.5 * b, a, b, f - 0.5 * d - 0.5 * e, d, e])
}

/// A world file's text for the affine (pixel corners), each number the
/// shortest that reads back.
pub fn write(affine: &[f64; 6]) -> String {
    let [x0, a, b, y0, c, d] = *affine;
    let centre_x = x0 + 0.5 * a + 0.5 * b;
    let centre_y = y0 + 0.5 * c + 0.5 * d;
    [a, c, b, d, centre_x, centre_y]
        .iter()
        .map(|v| format!("{v}\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_world_file_reads_to_corners_and_back() {
        let text = "0.5\n0.0\n0.0\n-0.5\n487000.25\n4420099.75\n";
        let affine = read(text).expect("reads");
        assert_eq!(affine, [487000.0, 0.5, 0.0, 4420100.0, 0.0, -0.5]);
        assert_eq!(read(&write(&affine)).expect("again"), affine);
        assert_eq!(extensions("TIF")[0], "tfw");
        assert_eq!(extensions(".png")[0], "pgw");
        assert_eq!(extension_for("jpeg"), "jgw");
    }
}
