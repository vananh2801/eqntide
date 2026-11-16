//! A per-user instance lock and local socket, independent of the Office bridge.
use crate::{Event, EventSender as Sender};
use fs2::FileExt;
use interprocess::local_socket::{prelude::*, GenericNamespaced, ListenerOptions, Stream};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
};

pub struct Instance {
    _lock: File,
}
pub fn acquire(args: &[String], tx: Sender<Event>) -> io::Result<Option<Instance>> {
    let dirs = directories::ProjectDirs::from("com", "eqntide", "EqnTide")
        .ok_or_else(|| io::Error::other("No user data directory"))?;
    std::fs::create_dir_all(dirs.data_local_dir())?;
    let lock_path = dirs.data_local_dir().join("desktop.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    // Hash the per-user path to keep Unix socket / Windows pipe names short.
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    lock_path.hash(&mut hasher);
    let name = format!("eqntide-desktop-{:x}", hasher.finish());
    if let Err(error) = lock.try_lock_exclusive() {
        let is_contended = error.kind() == io::ErrorKind::WouldBlock
            || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            || error.raw_os_error() == Some(33); // ERROR_LOCK_VIOLATION on Windows
        if !is_contended {
            return Err(error);
        }
        let mut last = None;
        for _ in 0..40 {
            match Stream::connect(name.clone().to_ns_name::<GenericNamespaced>()?) {
                Ok(mut stream) => {
                    serde_json::to_writer(&mut stream, args)?;
                    stream.write_all(b"\n")?;
                    return Ok(None);
                }
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
        return Err(
            last.unwrap_or_else(|| io::Error::other("Desktop did not accept the launch request")),
        );
    }
    let listener = ListenerOptions::new()
        .name(name.to_ns_name::<GenericNamespaced>()?)
        .try_overwrite(true)
        .create_sync()?;
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
    Ok(Some(Instance { _lock: lock }))
}
