//! Where a file is (docs/adr/0627 §8): the network file system a path is on (NFS, SMB, sshfs …), where SQLite's locks
//! do not hold, so that a GeoPackage there is not presented as a file several people may write at once. Linux reads its
//! mount table, macOS its `mount`'s list, Windows a UNC path; a mapped Windows drive is not told.

use std::path::{Path, PathBuf};

/// The network file systems, as Linux's mount table and macOS's `mount` name them.
const NETWORK: &[&str] = &[
    "nfs",
    "nfs4",
    "cifs",
    "smb3",
    "smbfs",
    "ncpfs",
    "afs",
    "9p",
    "ceph",
    "glusterfs",
    "lustre",
    "gpfs",
    "beegfs",
    "davfs",
    "afpfs",
    "webdav",
    "fuse.sshfs",
    "fuse.rclone",
    "fuse.s3fs",
    "fuse.gcsfuse",
    "fuse.davfs2",
    "fuse.glusterfs",
    "fuse.cephfs",
];

fn network(kind: &str) -> bool {
    NETWORK.contains(&kind)
}

/// The network file system `path` is on, its kind; none for a local one or when it cannot be told.
pub fn network_share(path: &Path) -> Option<String> {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if s.starts_with(r"\\?\UNC\") || (s.starts_with(r"\\") && !s.starts_with(r"\\?\")) {
            return Some("SMB".into());
        }
        None
    }
    #[cfg(target_os = "linux")]
    {
        let table = std::fs::read_to_string("/proc/self/mountinfo").ok()?;
        linux_kind(&table, &path).filter(|k| network(k))
    }
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("/sbin/mount").output().ok()?;
        mac_kind(&String::from_utf8_lossy(&out.stdout), &path).filter(|k| network(k))
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = path;
        None
    }
}

/// A mount table's path with its octal escapes (`\040` a space) read.
fn unescape(text: &str) -> PathBuf {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let digits: String = chars.clone().take(3).collect();
            if digits.len() == 3
                && digits.chars().all(|d| ('0'..='7').contains(&d))
                && let Ok(n) = u8::from_str_radix(&digits, 8)
            {
                out.push(char::from(n));
                for _ in 0..3 {
                    chars.next();
                }
                continue;
            }
        }
        out.push(c);
    }
    PathBuf::from(out)
}

/// The file system kind of the mount `path` is under, by Linux's `/proc/self/mountinfo`: the deepest mount point
/// that holds it.
pub fn linux_kind(table: &str, path: &Path) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for line in table.lines() {
        let fields: Vec<&str> = line.split(' ').collect();
        let Some(dash) = fields.iter().position(|f| *f == "-") else {
            continue;
        };
        let (Some(point), Some(kind)) = (fields.get(4), fields.get(dash + 1)) else {
            continue;
        };
        let point = unescape(point);
        if path.starts_with(&point) {
            let depth = point.components().count();
            if best.as_ref().is_none_or(|(d, _)| depth >= *d) {
                best = Some((depth, (*kind).to_owned()));
            }
        }
    }
    best.map(|(_, k)| k)
}

/// The file system kind of the mount `path` is under, by macOS's `mount` list (`source on point (kind, …)`).
pub fn mac_kind(list: &str, path: &Path) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for line in list.lines() {
        let Some(on) = line.find(" on ") else {
            continue;
        };
        let rest = &line[on + 4..];
        let Some(open) = rest.rfind(" (") else {
            continue;
        };
        let point = PathBuf::from(&rest[..open]);
        let kind = rest[open + 2..]
            .split([',', ')'])
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        if path.starts_with(&point) {
            let depth = point.components().count();
            if best.as_ref().is_none_or(|(d, _)| depth >= *d) {
                best = Some((depth, kind));
            }
        }
    }
    best.map(|(_, k)| k)
}

/// Whether a file system kind is a network one.
pub fn is_network(kind: &str) -> bool {
    network(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX: &str = "\
22 1 259:2 / / rw,relatime shared:1 - ext4 /dev/nvme0n1p2 rw
40 22 0:39 / /mnt/ortak rw,relatime shared:20 - nfs4 sunucu:/veri rw,vers=4.2
41 22 0:40 / /mnt/ofis\\040pay rw,relatime shared:21 - cifs //ofis/pay rw,vers=3.1.1
42 40 259:3 / /mnt/ortak/yerel rw,relatime shared:22 - ext4 /dev/sdb1 rw
43 22 0:41 / /home/kullanici/uzak rw,nosuid shared:23 - fuse.sshfs kullanici@sunucu: rw";

    #[test]
    fn the_deepest_mount_tells_the_file_system() {
        let kind = |p: &str| linux_kind(LINUX, Path::new(p));
        assert_eq!(
            kind("/home/kullanici/proje/parsel.gpkg").as_deref(),
            Some("ext4")
        );
        assert_eq!(kind("/mnt/ortak/parsel.gpkg").as_deref(), Some("nfs4"));
        assert_eq!(
            kind("/mnt/ortak/yerel/parsel.gpkg").as_deref(),
            Some("ext4")
        );
        assert_eq!(kind("/mnt/ofis pay/parsel.gpkg").as_deref(), Some("cifs"));
        assert_eq!(
            kind("/home/kullanici/uzak/a.gpkg").as_deref(),
            Some("fuse.sshfs")
        );
        // A mount point is whole components: /mnt/ortakx is not under /mnt/ortak.
        assert_eq!(kind("/mnt/ortakx/a.gpkg").as_deref(), Some("ext4"));
        assert!(is_network("nfs4") && is_network("cifs") && is_network("fuse.sshfs"));
        assert!(!is_network("ext4") && !is_network("btrfs") && !is_network("fuse.portal"));
    }

    #[test]
    fn macos_mount_list_tells_the_file_system() {
        let list = "\
/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)
//kullanici@ofis/pay on /Volumes/pay (smbfs, nodev, nosuid, mounted by kullanici)
sunucu:/veri on /Volumes/ortak veri (nfs, nodev, nosuid, mounted by kullanici)";
        let kind = |p: &str| mac_kind(list, Path::new(p));
        assert_eq!(kind("/Users/kullanici/a.gpkg").as_deref(), Some("apfs"));
        assert_eq!(kind("/Volumes/pay/proje/a.gpkg").as_deref(), Some("smbfs"));
        assert_eq!(kind("/Volumes/ortak veri/a.gpkg").as_deref(), Some("nfs"));
    }
}
