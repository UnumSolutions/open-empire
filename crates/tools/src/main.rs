use empire_content::{FORMAT_VERSION, Manifest, PackFile};
use empire_sim::{MapKind, Replay, World};
use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};
fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
fn arg(args: &[String], i: usize, name: &str) -> Result<String, String> {
    args.get(i).cloned().ok_or(format!("missing {name}"))
}
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("identity") => {
            println!(
                "{}",
                serde_json::to_string(&empire_sim::Identity::default())
                    .map_err(|e| e.to_string())?
            );
        }
        Some("simulate") => {
            let ticks = args
                .get(1)
                .map_or(Ok(1200), |s| s.parse::<u64>())
                .map_err(|e| e.to_string())?;
            let players = args
                .get(2)
                .map_or(Ok(2), |s| s.parse::<u8>())
                .map_err(|e| e.to_string())?;
            if ticks > 1_000_000 {
                return Err("maximum 1,000,000 ticks".into());
            }
            let mut world = World::new(42, players, MapKind::Land)?;
            let initial = world.clone();
            let mut commands = Vec::new();
            let start = std::time::Instant::now();
            while world.tick < ticks && !world.finished {
                let batch: Vec<_> = (0..players)
                    .flat_map(|p| empire_ai::commands(&world, p, empire_ai::Difficulty::Normal))
                    .collect();
                let errors = world.step(&batch);
                if !errors.is_empty() {
                    return Err(errors.join("; "));
                }
                commands.extend(batch);
            }
            println!(
                "ticks={} entities={} elapsed={:?} hash={}",
                world.tick,
                world.entities.len(),
                start.elapsed(),
                world.hash()
            );
            if let Some(path) = args.get(3) {
                let replay = Replay {
                    format: 1,
                    initial,
                    commands,
                    end_tick: world.tick,
                    final_hash: world.hash(),
                };
                fs::write(
                    path,
                    serde_json::to_vec(&replay).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                println!("replay written: {path}");
            }
        }
        Some("replay") => {
            let data = read_limited(Path::new(&arg(&args, 1, "replay path")?), 32 * 1024 * 1024)?;
            let replay: Replay = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            let w = replay.play()?;
            println!("verified tick={} hash={}", w.tick, w.hash());
        }
        Some("inventory-de") => {
            let root = PathBuf::from(arg(&args, 1, "DE installation path")?);
            let build = arg(&args, 2, "Steam build ID")?;
            let output = arg(&args, 3, "output manifest path")?;
            if !build.bytes().all(|b| b.is_ascii_digit()) || build.is_empty() {
                return Err("Steam build ID must be numeric".into());
            }
            let mut files = Vec::new();
            collect_files(&root, &root, &mut files)?;
            let manifest = Manifest {
                format: FORMAT_VERSION,
                ruleset: "de-unconverted".into(),
                source_build: Some(build),
                proprietary: true,
                files,
            };
            manifest.validate()?;
            fs::write(
                &output,
                serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "Inventoried {} files. This records provenance only; DE conversion is not implemented.",
                manifest.files.len()
            );
        }
        Some("verify-pack") => {
            let root = PathBuf::from(arg(&args, 1, "pack root")?);
            let bytes = read_limited(Path::new(&arg(&args, 2, "manifest")?), 32 * 1024 * 1024)?;
            let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            manifest.validate()?;
            for file in &manifest.files {
                let mut path = root.clone();
                for component in file.path.split('/') {
                    path.push(component);
                    if fs::symlink_metadata(&path)
                        .map_err(|e| e.to_string())?
                        .file_type()
                        .is_symlink()
                    {
                        return Err("symlinks are not allowed in packs".into());
                    }
                }
                let (size, hash) = hash_file(&path)?;
                if size != file.bytes || hash != file.blake3 {
                    return Err(format!("integrity mismatch: {}", file.path));
                }
            }
            println!(
                "Verified {} files; ruleset {}",
                manifest.files.len(),
                manifest.ruleset
            );
        }
        Some("relay") => {
            let addr = args.get(1).map(String::as_str).unwrap_or("127.0.0.1:47624");
            let players = args
                .get(2)
                .map_or(Ok(2), |s| s.parse::<u8>())
                .map_err(|e| e.to_string())?;
            relay(addr, players)?;
        }
        _ => println!(
            "Open Empire tools\n  simulate [ticks=1200] [players=2] [output.empire-replay]\n  replay <file>\n  inventory-de <installation> <steam-build-id> <output.json>\n  verify-pack <root> <manifest.json>\n  relay [127.0.0.1:47624] [players=2]"
        ),
    }
    Ok(())
}
fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("file exceeds size limit".into());
    }
    Ok(bytes)
}
fn hash_file(path: &Path) -> Result<(u64, String), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("expected a regular file".into());
    }
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        size += n as u64;
        if size > empire_content::MAX_PACK_BYTES {
            return Err("file too large".into());
        }
        hash.update(&buf[..n]);
    }
    Ok((size, hash.finalize().to_hex().to_string()))
}
fn collect_files(root: &Path, path: &Path, files: &mut Vec<PackFile>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!("symlink rejected: {}", path.display()));
    }
    if metadata.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            collect_files(root, &e.path(), files)?;
        }
    } else if metadata.is_file() {
        if files.len() >= 100_000 {
            return Err("too many files".into());
        }
        let (bytes, hash) = hash_file(path)?;
        let relative = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("path must be UTF-8")?
            .replace('\\', "/");
        files.push(PackFile {
            path: relative,
            bytes,
            blake3: hash,
        });
    } else {
        return Err("special file rejected".into());
    }
    Ok(())
}
fn send(stream: &mut TcpStream, message: &empire_net::ServerMessage) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    stream.write_all(&bytes).map_err(|e| e.to_string())
}
fn read_message(reader: &mut BufReader<TcpStream>) -> Result<empire_net::ClientMessage, String> {
    let mut bytes = Vec::new();
    reader
        .take(empire_net::MAX_FRAME_BYTES as u64 + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err("peer disconnected".into());
    }
    if bytes.len() > empire_net::MAX_FRAME_BYTES || bytes.last() != Some(&b'\n') {
        return Err("oversized or truncated frame".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn relay(addr: &str, players: u8) -> Result<(), String> {
    use empire_net::{ClientMessage, Lockstep, ServerMessage};
    let mut room = Lockstep::new(Default::default(), players)?;
    let listener = TcpListener::bind(addr).map_err(|e| e.to_string())?;
    println!(
        "Development LAN relay listening on {} for {players} clients",
        listener.local_addr().unwrap()
    );
    let (tx, rx) = mpsc::sync_channel(128);
    let mut clients = std::collections::BTreeMap::new();
    while !room.ready() {
        let (stream, _) = listener.accept().map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut stream = stream;
        let hello = match read_message(&mut reader) {
            Ok(ClientMessage::Hello(h)) => h,
            _ => {
                let _ = send(&mut stream, &ServerMessage::Error("hello required".into()));
                continue;
            }
        };
        let player = hello.player;
        if let Err(e) = room.join(hello) {
            let _ = send(&mut stream, &ServerMessage::Error(e));
            continue;
        }
        send(&mut stream, &ServerMessage::Welcome { player, players })?;
        reader
            .get_mut()
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| e.to_string())?;
        clients.insert(player, stream);
        let tx = tx.clone();
        std::thread::spawn(move || {
            loop {
                let m = read_message(&mut reader);
                let failed = m.is_err();
                if tx.send((player, m)).is_err() || failed {
                    break;
                }
            }
        });
    }
    for stream in clients.values_mut() {
        send(stream, &ServerMessage::Ready)?;
    }
    loop {
        let (p, message) = rx
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "match timed out".to_string())?;
        let outgoing = match message? {
            ClientMessage::Submit(turn) => match room.submit(p, turn) {
                Ok(Some(t)) => Some(ServerMessage::Turn(t)),
                Ok(None) => None,
                Err(e) => {
                    send(clients.get_mut(&p).unwrap(), &ServerMessage::Error(e))?;
                    None
                }
            },
            ClientMessage::Hash { tick, hash } => match room.report_hash(p, tick, hash) {
                Ok(m) => m,
                Err(e) => {
                    send(clients.get_mut(&p).unwrap(), &ServerMessage::Error(e))?;
                    None
                }
            },
            ClientMessage::Hello(_) => {
                send(
                    clients.get_mut(&p).unwrap(),
                    &ServerMessage::Error("already joined".into()),
                )?;
                None
            }
        };
        if let Some(message) = outgoing {
            for stream in clients.values_mut() {
                send(stream, &message)?;
            }
        }
    }
}
