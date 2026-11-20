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

fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn open_private_rw(path: &Path) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

fn open_lock_in(dir: &Path) -> io::Result<(File, PathBuf)> {
    ensure_private_dir(dir)?;
    let lock_path = dir.join("desktop.lock");
    match open_private_rw(&lock_path) {
        Ok(file) => Ok((file, lock_path)),
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
            // A previous elevated run or read-only lock file inside the user's
            // own data directory can block write access.
            let _ = std::fs::remove_file(&lock_path);
            let file = open_private_rw(&lock_path)?;
            Ok((file, lock_path))
        }
        Err(err) => Err(err),
    }
}

fn open_lock_file_in(dir: &Path) -> io::Result<(File, PathBuf)> {
    open_lock_in(dir).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("Failed to open lock file in '{}': {err}", dir.display()),
        )
    })
}

fn primary_data_dir() -> io::Result<PathBuf> {
    directories::ProjectDirs::from("com", "eqntide", "EqnTide")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Failed to locate per-user EqnTide local data directory",
            )
        })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IpcEnvelope {
    secret: String,
    args: Vec<String>,
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn load_or_create_ipc_secret(dir: &Path) -> io::Result<String> {
    ensure_private_dir(dir)?;
    let secret_path = dir.join("ipc.secret");
    if let Ok(raw) = std::fs::read_to_string(&secret_path) {
        let trimmed = raw.trim();
        if trimmed.len() >= 16 && trimmed.bytes().all(|b| b.is_ascii_graphic()) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&secret_path, std::fs::Permissions::from_mode(0o600));
            }
            return Ok(trimmed.to_owned());
        }
    }
    let secret = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let mut opts = OpenOptions::new();
    opts.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(&secret_path)?;
    file.write_all(secret.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(secret)
}

fn verify_ipc_payload(line: &str, expected_secret: &str) -> Option<Vec<String>> {
    let envelope: IpcEnvelope = serde_json::from_str(line).ok()?;
    if constant_time_eq(envelope.secret.as_bytes(), expected_secret.as_bytes()) {
        Some(envelope.args)
    } else {
        None
    }
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
    let primary_dir = primary_data_dir()?;
    acquire_in(&primary_dir, args, tx)
}

fn acquire_in(dir: &Path, args: &[String], tx: Sender<Event>) -> io::Result<Option<Instance>> {
    let (lock, lock_path) = open_lock_file_in(dir)?;
    acquire_with_lock(lock, lock_path, args, tx)
}

fn acquire_with_lock(
    lock: File,
    lock_path: PathBuf,
    args: &[String],
    tx: Sender<Event>,
) -> io::Result<Option<Instance>> {
    let state_dir = lock_path
        .parent()
        .ok_or_else(|| io::Error::other("lock path has no parent directory"))?;
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
                    let secret = load_or_create_ipc_secret(state_dir)?;
                    let payload = IpcEnvelope {
                        secret,
                        args: args.to_vec(),
                    };
                    serde_json::to_writer(&mut stream, &payload)?;
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

    let ipc_secret = load_or_create_ipc_secret(state_dir)?;
    let (listener, active_name) = bind_listener(&base_name)?;
    let _ = std::fs::write(&endpoint_path, &active_name);

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            // Limit malformed local clients; read each connection separately.
            let events = tx.clone();
            let secret = ipc_secret.clone();
            std::thread::spawn(move || {
                let mut line = String::new();
                if BufReader::new(stream.take(1024 * 1024))
                    .read_line(&mut line)
                    .is_ok()
                {
                    if let Some(args) = verify_ipc_payload(&line, &secret) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn verify_ipc_payload_rejects_unauthenticated_and_wrong_secret_messages() {
        let secret = "0123456789abcdef0123456789abcdef";
        assert!(verify_ipc_payload(r#"["eqntide://open"]"#, secret).is_none());
        assert!(
            verify_ipc_payload(r#"{"secret":"wrong","args":["eqntide://open"]}"#, secret).is_none()
        );
        let accepted = verify_ipc_payload(
            &serde_json::to_string(&IpcEnvelope {
                secret: secret.into(),
                args: vec!["eqntide://open?tab=batch".into()],
            })
            .unwrap(),
            secret,
        );
        assert_eq!(accepted, Some(vec!["eqntide://open?tab=batch".into()]));
    }

    #[test]
    fn acquire_in_rejects_unauthenticated_socket_messages_and_accepts_second_instance() {
        let dir = tempfile::tempdir().unwrap();
        let (raw_tx, rx) = mpsc::channel();
        let sender = Sender::new(raw_tx);
        let _primary = acquire_in(dir.path(), &[], sender.clone())
            .unwrap()
            .unwrap();

        // 1. Unauthenticated raw Vec<String> write to the active socket must be ignored.
        let endpoint = read_endpoint_name(&dir.path().join("desktop.endpoint"), "");
        assert!(!endpoint.is_empty());
        let mut rogue =
            Stream::connect(endpoint.to_ns_name::<GenericNamespaced>().unwrap()).unwrap();
        rogue
            .write_all(b"[\"eqntide://open?tab=batch\"]\n")
            .unwrap();
        drop(rogue);
        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err());

        // 2. Legitimate second instance sharing the per-user state dir delivers its args.
        let secondary = acquire_in(
            dir.path(),
            &["eqntide://open?tab=batch".to_string()],
            sender,
        )
        .unwrap();
        assert!(secondary.is_none());
        match rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap() {
            Event::Launch(args) => assert_eq!(args, vec!["eqntide://open?tab=batch"]),
            _ => panic!("expected Event::Launch"),
        }
    }
}
