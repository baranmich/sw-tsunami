//! Fork: saving over an existing document inside the macOS App Sandbox (Mac App Store builds).
//!
//! The sandbox lets the app write the document the user opened or picked, but not create files
//! beside it, so the safe save's sibling temporary file is refused (`safe_save`). Upstream then
//! refuses the save rather than write in place, so the only copy is never at risk. Sandboxed, that
//! would refuse every Save, so this writes in place with the same guarantee kept another way:
//! the current file is copied into the app's own container first, the new content is written and
//! synced, and on any failure the copy is written back. The copy is removed after a good save and
//! kept (and named in the error) if even the restore fails.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Whether this process runs in the macOS App Sandbox (the system sets this for sandboxed apps).
pub(crate) fn sandboxed() -> bool {
    cfg!(target_os = "macos") && std::env::var_os("APP_SANDBOX_CONTAINER_ID").is_some()
}

/// Replaces the content of the existing file at `path` with what `prepare` writes, keeping a copy
/// of the old content in `backups` until the new content is safely on disk.
pub(crate) fn in_place(path: &Path, backups: &Path, prepare: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    let backup = backup_path(path, backups)?;
    fs::copy(path, &backup).map_err(|e| io::Error::new(e.kind(), format!("couldn't keep a copy of the document before saving ({e})")))?;
    let written = OpenOptions::new().write(true).truncate(true).open(path).and_then(|mut file| {
        prepare(&mut file)?;
        file.flush()?;
        file.sync_all()
    });
    match written {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
            Ok(())
        }
        Err(failed) => match restore(&backup, path) {
            Ok(()) => {
                let _ = fs::remove_file(&backup);
                Err(io::Error::new(failed.kind(), format!("{failed}; the document was left as it was")))
            }
            Err(e) => Err(io::Error::new(
                failed.kind(),
                format!("{failed}; restoring the document also failed ({e}): its previous version is kept at {}", backup.display()),
            )),
        },
    }
}

/// The folder for those copies: inside the app's container when sandboxed (the temporary folder
/// there is the container's own).
pub(crate) fn default_backups() -> PathBuf {
    std::env::temp_dir().join(format!("{}-save-backups", wordcraft_brand::DATA_DIR))
}

fn backup_path(path: &Path, backups: &Path) -> io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fs::create_dir_all(backups)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "document".into());
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    Ok(backups.join(format!("{}-{id}-{name}", std::process::id())))
}

fn restore(backup: &Path, path: &Path) -> io::Result<()> {
    let old = fs::read(backup)?;
    let mut file = OpenOptions::new().write(true).truncate(true).open(path)?;
    file.write_all(&old)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sandbox-save-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn replaces_the_content_and_leaves_no_copy() {
        let d = scratch("ok");
        let doc = d.join("letter.docx");
        fs::write(&doc, b"old").unwrap();
        let backups = d.join("backups");
        in_place(&doc, &backups, |f| f.write_all(b"new content")).unwrap();
        assert_eq!(fs::read(&doc).unwrap(), b"new content");
        assert_eq!(fs::read_dir(&backups).unwrap().count(), 0, "the copy goes after a good save");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_failed_write_puts_the_old_content_back() {
        let d = scratch("fail");
        let doc = d.join("letter.docx");
        fs::write(&doc, b"the only copy").unwrap();
        let backups = d.join("backups");
        let e = in_place(&doc, &backups, |f| {
            f.write_all(b"half")?;
            Err(io::Error::other("disk full"))
        })
        .unwrap_err();
        assert!(e.to_string().contains("left as it was"), "{e}");
        assert_eq!(fs::read(&doc).unwrap(), b"the only copy");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_shorter_document_leaves_no_old_tail() {
        let d = scratch("short");
        let doc = d.join("notes.txt");
        fs::write(&doc, b"a much longer old text").unwrap();
        in_place(&doc, &d.join("b"), |f| f.write_all(b"short")).unwrap();
        assert_eq!(fs::read(&doc).unwrap(), b"short");
        fs::remove_dir_all(&d).unwrap();
    }
}
