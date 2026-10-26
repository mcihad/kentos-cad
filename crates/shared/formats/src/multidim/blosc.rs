//! Blosc frames (docs/adr/0645 §3), as c-blosc 1.x writes them and numcodecs' Blosc (Zarr's default compressor) and
//! HDF5's filter 32001 keep them: a sixteen-byte header (format version, codec version, flags, element size, the
//! plain size, the block size, the frame's size), each block's start, each block cut into one stream per byte of an
//! element (when split) compressed by BloscLZ, LZ4, Snappy, zlib or Zstandard; a stream as long as its plain size is
//! stored as it is. A block is then put back from its byte or bit shuffle.

const MAX_SPLITS: usize = 16;
const MIN_BUFFER: usize = 128;
const MAX_DISTANCE: usize = 8191;

fn le32(b: &[u8], at: usize) -> Result<u32, String> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| "Blosc çerçevesi eksik".into())
}

/// A Blosc frame's plain bytes.
pub fn decompress(frame: &[u8]) -> Result<Vec<u8>, String> {
    if frame.len() < 16 {
        return Err("Blosc çerçevesi 16 bayttan kısa".into());
    }
    let flags = frame[2];
    let typesize = usize::from(frame[3]).max(1);
    let nbytes = le32(frame, 4)? as usize;
    let blocksize = le32(frame, 8)? as usize;
    let cbytes = le32(frame, 12)? as usize;
    if nbytes > (1 << 31) || cbytes > frame.len() {
        return Err("Blosc başlığı bozuk".into());
    }
    if flags & 0x02 != 0 {
        // Stored as it is.
        return frame
            .get(16..16 + nbytes)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| "Blosc çerçevesi eksik".into());
    }
    if nbytes == 0 {
        return Ok(Vec::new());
    }
    if blocksize == 0 {
        return Err("Blosc başlığında blok boyu 0".into());
    }
    let codec = (flags >> 5) & 0x7;
    let dont_split = flags & 0x10 != 0;
    let blocks = nbytes.div_ceil(blocksize);
    let mut out = Vec::with_capacity(nbytes);
    for b in 0..blocks {
        let start = le32(frame, 16 + 4 * b)? as usize;
        let bsize = blocksize.min(nbytes - b * blocksize);
        let leftover = bsize < blocksize;
        let splits =
            if !dont_split && typesize <= MAX_SPLITS && bsize / typesize >= MIN_BUFFER && !leftover
            {
                typesize
            } else {
                1
            };
        let per = bsize / splits;
        let mut block = Vec::with_capacity(bsize);
        let mut at = start;
        for _ in 0..splits {
            let n = le32(frame, at)? as usize;
            at += 4;
            let src = frame.get(at..at + n).ok_or("Blosc akışı eksik")?;
            at += n;
            if n == per {
                block.extend_from_slice(src);
            } else {
                block.extend(stream(codec, src, per)?);
            }
        }
        block.resize(bsize, 0);
        let block = if flags & 0x04 != 0 {
            bit_unshuffle(&block, typesize)
        } else if flags & 0x01 != 0 {
            super::chunked::unshuffle(&block, typesize)
        } else {
            block
        };
        out.extend(block);
    }
    Ok(out)
}

/// One compressed stream of `size` plain bytes.
fn stream(codec: u8, src: &[u8], size: usize) -> Result<Vec<u8>, String> {
    match codec {
        0 => blosclz(src, size),
        1 => crate::compress::lz4::decompress_block(src, size),
        2 => crate::compress::snappy::decompress(src, size),
        3 => crate::raster::codec::inflate(src, size).map_err(|e| e.0),
        4 => crate::compress::zstd::decompress(src, size),
        c => Err(format!("Blosc'un {c} numaralı sıkıştırması bilinmiyor")),
    }
}

/// BloscLZ (FastLZ's level-1 kin): literal runs and matches back up to 8191 bytes, farther with a 16-bit distance.
pub fn blosclz(input: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::with_capacity(size);
    if input.is_empty() {
        return Ok(out);
    }
    let bad = || "BloscLZ akışı bozuk".to_owned();
    let mut ip = 0usize;
    let mut ctrl = usize::from(input[ip] & 31);
    ip += 1;
    loop {
        if ctrl >= 32 {
            let mut len = (ctrl >> 5) - 1;
            let mut ofs = (ctrl & 31) << 8;
            if len == 6 {
                loop {
                    let code = *input.get(ip).ok_or_else(bad)?;
                    ip += 1;
                    len += usize::from(code);
                    if code != 255 {
                        break;
                    }
                }
            }
            let code = usize::from(*input.get(ip).ok_or_else(bad)?);
            ip += 1;
            len += 3;
            let mut back = ofs + code;
            if code == 255 && ofs == 31 << 8 {
                let hi = usize::from(*input.get(ip).ok_or_else(bad)?);
                let lo = usize::from(*input.get(ip + 1).ok_or_else(bad)?);
                ip += 2;
                ofs = (hi << 8) + lo;
                back = ofs + MAX_DISTANCE;
            }
            // `ref` is op − back − 1.
            let from = out.len().checked_sub(back + 1).ok_or_else(bad)?;
            if out.len() + len > size {
                return Err(bad());
            }
            for k in 0..len {
                let v = out[from + k];
                out.push(v);
            }
        } else {
            let n = ctrl + 1;
            let src = input.get(ip..ip + n).ok_or_else(bad)?;
            if out.len() + n > size {
                return Err(bad());
            }
            out.extend_from_slice(src);
            ip += n;
        }
        if ip >= input.len() {
            break;
        }
        ctrl = usize::from(input[ip]);
        ip += 1;
    }
    if out.len() != size {
        return Err(format!(
            "BloscLZ {} bayt açtı, {size} bekleniyordu",
            out.len()
        ));
    }
    Ok(out)
}

/// Bits laid plane by plane (bitshuffle: the elements' bit `j` of byte `b` together, LSB first) made element by
/// element again; a tail of fewer than eight elements is kept as it is.
pub fn bit_unshuffle(b: &[u8], size: usize) -> Vec<u8> {
    let count = b.len() / size;
    let n = count - count % 8;
    let mut out = vec![0u8; b.len()];
    let plane = n / 8;
    for byte in 0..size {
        for bit in 0..8 {
            let p = &b[(byte * 8 + bit) * plane..(byte * 8 + bit + 1) * plane];
            for i in 0..n {
                if p[i / 8] >> (i % 8) & 1 == 1 {
                    out[i * size + byte] |= 1 << bit;
                }
            }
        }
    }
    out[n * size..].copy_from_slice(&b[n * size..]);
    out
}
