//! Rebuild the codec-produced binary samples (not an independent DWG oracle).
//! Geometry expectations are generated separately from the ASCII reference.
use kentos_contracts::DwgVersion;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1");
    let out = root.join("dwg");
    std::fs::create_dir_all(&out)?;
    for (name, source) in [
        ("mm", "dxf-units/mm.dxf"),
        ("empty", "dxf-import/empty.dxf"),
    ] {
        let bytes = kentos_dwg::encode(&std::fs::read(root.join(source))?, DwgVersion::AC1032)?;
        std::fs::write(out.join(format!("{name}.dwg")), bytes)?;
    }
    std::fs::write(out.join("unsupported.dwg"), b"AC1033\0\0\0")?;
    std::fs::write(out.join("truncated.dwg"), b"AC1032\0\0\0")?;
    Ok(())
}
