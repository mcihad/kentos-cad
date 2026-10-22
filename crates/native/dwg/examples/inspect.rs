//! Write the pinned codec's DXF bridge for investigation; never independent validation.
use std::io::Cursor;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let source = args.get(1).ok_or("source.dwg required")?;
    let target = args.get(2).ok_or("bridge.dxf required")?;
    let document = acadrust::DwgReader::from_stream(Cursor::new(std::fs::read(source)?)).read()?;
    std::fs::write(target, acadrust::DxfWriter::new(&document).write_to_vec()?)?;
    Ok(())
}
