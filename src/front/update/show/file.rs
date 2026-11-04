// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Files that hold shows and their unsaved changes: the kept file of each
//! show, and writing and removing files in place

use std::{
    fs,
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

use super::EXTENSION;

/// Returns the file in `unsaved_directory` that keeps the unsaved changes
/// of the show in `file`, or of a new show without one: by the path of the
/// show's file under `shows`, in which `..` takes away the directory before
/// it, or as `untitled.cinq.toml` for a new show
pub(super) fn kept_file(unsaved_directory: &Path, file: Option<&Path>) -> PathBuf {
    match file {
        Some(file) => unsaved_directory.join("shows").join(file.components().fold(
            PathBuf::new(),
            |path, component| match component {
                Component::Normal(name) => path.join(name),
                Component::ParentDir => path.parent().map_or_else(PathBuf::new, Path::to_owned),
                _ => path,
            },
        )),
        None => unsaved_directory.join(format!("untitled.{EXTENSION}")),
    }
}

/// Writes `text` to a file next to `file`, which then takes its place, so
/// that a failure halfway leaves the file as it was.  The file keeps its
/// permissions, and a link to it stays a link, since the text goes to the
/// file that the link names.  A new file gets the permissions of any file
/// that the program creates.  Waiting for the disk, as saving a show does,
/// makes the text outlast a power cut as well.
pub(super) fn write(file: &Path, text: &str, wait_for_disk: bool) -> io::Result<()> {
    let file = fs::canonicalize(file).unwrap_or_else(|_| file.to_owned());
    let directory = file.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(directory)?;
    let temporary = created_in(directory)?;
    if let Ok(metadata) = fs::metadata(&file) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())?;
    }
    temporary.as_file().write_all(text.as_bytes())?;
    if wait_for_disk {
        temporary.as_file().sync_all()?;
    }
    temporary.persist(file).map(drop).map_err(io::Error::from)
}

/// Returns a new, empty file in `directory` that asks for the permissions
/// any created file asks for, so that the umask narrows them as it does for
/// every other file, where tempfile on its own makes the file private
#[cfg(unix)]
fn created_in(directory: &Path) -> io::Result<tempfile::NamedTempFile> {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o666))
        .tempfile_in(directory)
}

/// Returns a new, empty file in `directory`
#[cfg(not(unix))]
fn created_in(directory: &Path) -> io::Result<tempfile::NamedTempFile> {
    tempfile::NamedTempFile::new_in(directory)
}

/// Removes a file, which is no failure when it is gone already
pub(super) fn remove(file: &Path) -> io::Result<()> {
    match fs::remove_file(file) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
