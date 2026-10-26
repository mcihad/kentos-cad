//! A host's network for a project's addressed files (docs/adr/0645 §5): a file at an address read by ranges, a store's
//! object whole. Only the shape: the desktop and the MCP server fill it with the cloud's HTTP client, the Python module
//! with Python's own; the cores that read a NetCDF, a raster or a point cloud take whatever the host gives.

/// A file at an address read by ranges.
pub trait RangeRead: Send + Sync {
    fn size(&self) -> u64;
    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String>;
}

/// A host's network.
pub trait Remote: Send + Sync {
    /// The address's file, its size known (a server that does not read by ranges refused, saying why).
    fn open(&self, url: &str) -> Result<Box<dyn RangeRead>, String>;
    /// An object at an address whole; none when the server has none there (404, or an S3 bucket's 403).
    fn object(&self, url: &str) -> Result<Option<Vec<u8>>, String>;
}
