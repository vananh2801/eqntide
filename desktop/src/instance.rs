//! A per-user instance lock and local socket, independent of the Office bridge.
use crate::{Event, EventSender as Sender};
use fs2::FileExt;
use interprocess::local_socket::{
    prelude::*, GenericNamespaced, Listener, ListenerOptions, Stream,
};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

pub struct Instance {
    _lock: File,
    endpoint_path: Option<PathBuf>,
}

impl Drop for Instance {
    fn drop(&mut self) {
        if let Some(path) = &self.endpoint_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn is_contended(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::WouldBlock
        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
        || error.raw_os_error() == Some(33) // ERROR_LOCK_VIOLATION on Windows
}

fn open_lock_in(dir: &Path) -> io::Result<(File, PathBuf)> {
    std::fs::create_dir_all(dir)?;
    let lock_path = dir.join("desktop.lock");
    match OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
    {
        Ok(file) => Ok((file, lock_path)),
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
            // A previous elevated run or read-only lock file can block write access.
            let _ = std::fs::remove_file(&lock_path);
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&lock_path)?;
            Ok((file, lock_path))
        }
        Err(err) => Err(err),
    }
}

fn open_lock_file() -> io::Result<(File, PathBuf)> {
    let primary_dir = directories::ProjectDirs::from("com", "eqntide", "EqnTide")
        .map(|dirs| dirs.data_local_dir().to_path_buf());
    if let Some(dir) = primary_dir {
        match open_lock_in(&dir) {
            Ok(res) => return Ok(res),
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {}
            Err(err) => {
                return Err(io::Error::new(
                    err.kind(),
                    format!("Failed to open lock file in '{}': {err}", dir.display()),
                ));
            }
        }
    }
    let fallback_dir = std::env::temp_dir().join("EqnTide");
    open_lock_in(&fallback_dir).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "Failed to open lock file in '{}': {err}",
                fallback_dir.display()
            ),
        )
    })
}

fn read_endpoint_name(endpoint_path: &Path, fallback: &str) -> String {
    std::fs::read_to_string(endpoint_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn try_bind_listener(name: &str) -> io::Result<Listener> {
    ListenerOptions::new()
        .name(name.to_ns_name::<GenericNamespaced>()?)
        .try_overwrite(true)
        .create_sync()
}

fn bind_listener(base_name: &str) -> io::Result<(Listener, String)> {
    // On Windows, interprocess creates named pipes with FILE_FLAG_FIRST_PIPE_INSTANCE,
    // which returns ERROR_ACCESS_DENIED (os error 5) if any handle to the same pipe
    // name still exists (for example while an exiting instance finishes tearing down).
    let mut last_err = None;
    for attempt in 0..4 {
        match try_bind_listener(base_name) {
            Ok(listener) => return Ok((listener, base_name.to_string())),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::AddrInUse
                ) =>
            {
                last_err = Some(err);
                if attempt < 3 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            Err(err) => {
                return Err(io::Error::new(
                    err.kind(),
                    format!("Failed to create IPC listener '{base_name}': {err}"),
                ));
            }
        }
    }

    // Fall back to a unique per-process pipe name; we hold the exclusive file lock
    // and publish the active pipe name in desktop.endpoint for secondary launches.
    let pid = std::process::id();
    for attempt in 0..8 {
        let candidate = format!("{base_name}-{pid}-{attempt}");
        match try_bind_listener(&candidate) {
            Ok(listener) => return Ok((listener, candidate)),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::AddrInUse
                ) =>
            {
                last_err = Some(err);
            }
            Err(err) => {
                return Err(io::Error::new(
                    err.kind(),
                    format!("Failed to create IPC listener '{candidate}': {err}"),
                ));
            }
        }
    }

    let err = last_err.unwrap_or_else(|| io::Error::other("Unknown IPC listener error"));
    Err(io::Error::new(
        err.kind(),
        format!("Failed to create IPC listener '{base_name}': {err}"),
    ))
}

pub fn acquire(args: &[String], tx: Sender<Event>) -> io::Result<Option<Instance>> {
    let (lock, lock_path) = open_lock_file()?;
    let endpoint_path = lock_path.with_file_name("desktop.endpoint");

    // Hash the per-user path to keep Unix socket / Windows pipe names short.
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    lock_path.hash(&mut hasher);
    let base_name = format!("eqntide-desktop-{:x}", hasher.finish());

    if let Err(error) = lock.try_lock_exclusive() {
        if !is_contended(&error) {
            return Err(io::Error::new(
                error.kind(),
                format!("Failed to acquire lock '{}': {error}", lock_path.display()),
            ));
        }

        let mut acquired_after_wait = false;
        let mut last: Option<(String, io::Error)> = None;
        for _ in 0..40 {
            let target_name = read_endpoint_name(&endpoint_path, &base_name);
            match Stream::connect(target_name.clone().to_ns_name::<GenericNamespaced>()?) {
                Ok(mut stream) => {
                    serde_json::to_writer(&mut stream, args)?;
                    stream.write_all(b"\n")?;
                    return Ok(None);
                }
                Err(e) => {
                    last = Some((target_name, e));
                }
            }

            if lock.try_lock_exclusive().is_ok() {
                acquired_after_wait = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if !acquired_after_wait {
            return Err(match last {
                Some((pipe_name, err)) if err.kind() == io::ErrorKind::PermissionDenied => {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "EqnTide is already running, but access to its IPC pipe '{pipe_name}' was denied ({err}). Switch to the open EqnTide window or close it from Task Manager first."
                        ),
                    )
                }
                Some((pipe_name, err)) => io::Error::new(
                    err.kind(),
                    format!("Failed to connect to running EqnTide instance ('{pipe_name}'): {err}"),
                ),
                None => io::Error::other("Desktop did not accept the launch request"),
            });
        }
    }

    let (listener, active_name) = bind_listener(&base_name)?;
    let _ = std::fs::write(&endpoint_path, &active_name);

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            // Limit malformed local clients; read each connection separately.
            let events = tx.clone();
            std::thread::spawn(move || {
                let mut line = String::new();
                if BufReader::new(stream.take(1024 * 1024))
                    .read_line(&mut line)
                    .is_ok()
                {
                    if let Ok(args) = serde_json::from_str::<Vec<String>>(&line) {
                        let _ = events.send(Event::Launch(args));
                    }
                }
            });
        }
    });

    Ok(Some(Instance {
        _lock: lock,
        endpoint_path: Some(endpoint_path),
    }))
}
