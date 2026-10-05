//! Crash-safe file replacement: write a sibling temp file, fsync, rename.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Replaces `path` with `contents` so readers see either the old or the new
/// file, never a partial one. An existing file keeps its permissions.
pub fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let existing_mode = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    let temp_path = temp_sibling(path);

    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o644);
        let mut file = options.open(&temp_path)?;
        file.write_all(contents)?;
        if let Some(permissions) = existing_mode {
            file.set_permissions(permissions)?;
        }
        file.sync_all()?;
        replace(&temp_path, path)?;
        sync_folder(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(windows))]
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

/// Windows refuses to replace a file another program has open, often
/// only for a moment (a virus scanner, a thumbnailer), so try a few times
/// before saying so.
#[cfg(windows)]
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    let mut tries = 0;
    loop {
        match fs::rename(from, to) {
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied && tries < 10 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                return Err(io::Error::new(
                    error.kind(),
                    format!("{} is open in another program", to.display()),
                ));
            }
            result => return result,
        }
    }
}

/// Makes the rename itself durable. Windows cannot open a folder as a
/// file, and its renames are written through by NTFS.
#[cfg(unix)]
fn sync_folder(folder: &Path) -> io::Result<()> {
    match fs::File::open(folder) {
        Ok(folder) => folder.sync_all(),
        // macOS lets an app opened on a file in a protected folder, such
        // as Documents, replace the file but not open the folder. The
        // rename has happened by then, so the save stands.
        Err(error)
            if cfg!(target_os = "macos") && error.kind() == io::ErrorKind::PermissionDenied =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

#[cfg(not(unix))]
fn sync_folder(_folder: &Path) -> io::Result<()> {
    Ok(())
}

fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(".{name}.triib-{}-{unique}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.toml");
        write(&path, b"first").unwrap();
        write(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        let leftovers: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "no temp files left behind");
    }

    #[cfg(unix)]
    #[test]
    fn keeps_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private.toml");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        write(&path, b"new").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn failure_leaves_original_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing-dir").join("file.toml");
        assert!(write(&path, b"data").is_err());
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        assert!(
            write(&target, b"data").is_err(),
            "cannot replace a directory"
        );
        assert!(target.is_dir());
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            1,
            "temp file cleaned up"
        );
    }
}
