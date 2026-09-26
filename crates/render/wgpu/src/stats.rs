//! What the last frame of a view cost (TODOS.md REN-01): the host shows it,
//! and measurements read it. Counts are of what was drawn, bytes of GPU
//! buffers and textures.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// Straight pieces drawn (curves count by their chords).
    pub segments: u64,
    /// Fill triangles drawn.
    pub triangles: u64,
    /// Point marks drawn.
    pub markers: u64,
    /// Draw calls issued, the background included.
    pub draw_calls: u32,
    /// Buffer bytes uploaded while preparing the frame: the frame uniform, plus
    /// any scene part the view had not seen. 64 when nothing but the view moved.
    pub uploaded_bytes: u64,
    /// Buffer bytes the view holds on the GPU.
    pub resident_bytes: u64,
    /// Frames the view has been prepared for since it appeared.
    pub frames: u64,
    /// Samples per pixel the frame is drawn with (TODOS.md AA-01).
    pub samples: u32,
    /// Bytes of the view's own targets (multisampled colour and picture); 0
    /// when it draws straight into the host's pass.
    pub target_bytes: u64,
}
