//! Two loaded game processes exchange live input/checksums through LoopbackNet.
use std::{
    error::Error,
    fs,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use uqm_engine::{
    host::{Connection, Net},
    network::loopback::LoopbackNet,
};

use uqm_rust::supermelee::netplay::packet::{DecodeError, Packet};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
struct Replay(Child);
impl Drop for Replay {
    fn drop(&mut self) {
        if self
            .0
            .try_wait()
            .unwrap_or_else(|error| panic!("query replay process: {error}"))
            .is_none()
        {
            self.0
                .kill()
                .unwrap_or_else(|error| panic!("kill owned replay process: {error}"));
        }
        self.0
            .wait()
            .unwrap_or_else(|error| panic!("reap replay process: {error}"));
    }
}
fn start_replay(root: &Path, peer: usize, port: u16) -> Result<Replay> {
    let directory = root.join(peer.to_string());
    fs::create_dir_all(directory.join("config"))?;
    let log = fs::File::create(directory.join("game.log"))?;
    let mut script: serde_json::Value =
        serde_json::from_str(include_str!("../scripts/super-melee-v1.json"))?;
    let steps = script["steps"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("scenario steps missing"));
    let at = steps
        .iter()
        .position(|step| step["label"] == "melee-one-ship-each")
        .unwrap_or_else(|| panic!("scenario capture not found"));
    steps.insert(at, serde_json::json!({"action":"connect_netplay", "remote_player":1-peer, "server":peer==1, "port":port}));
    fs::write(
        directory.join("scenario.json"),
        serde_json::to_vec(&script)?,
    )?;
    Ok(Replay(
        Command::new(env!("CARGO_BIN_EXE_uqm"))
            .arg("--replay")
            .arg(directory.join("scenario.json"))
            .arg("--replay-summary")
            .arg(directory.join("summary.json"))
            .arg("--automation-output")
            .arg(directory.join("automation"))
            .arg("--configdir")
            .arg(directory.join("config"))
            .arg("--contentdir")
            .arg(root.join("content"))
            .args(["--res", "320x240", "--scale", "none"])
            .env("SDL_VIDEODRIVER", "dummy")
            .env("SDL_AUDIODRIVER", "dummy")
            .env("UQM_PARITY", "1")
            .env_remove("UQM_REPLAY_JITTER")
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?,
    ))
}
fn battle_checksums(trace: &str, fields: &str) -> Vec<u32> {
    let frames: std::collections::HashSet<_> = fields
        .lines()
        .filter_map(|line| {
            let (step, json) = line
                .split_once(' ')
                .unwrap_or_else(|| panic!("malformed parity fields"));
            let fields: serde_json::Value =
                serde_json::from_str(json).unwrap_or_else(|error| panic!("parity fields: {error}"));
            (fields["mode.boundary"] == "battle").then_some(step)
        })
        .collect();
    trace
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            if words.next()? != "S" {
                return None;
            }
            let step = words.next()?;
            let activity = words
                .next()?
                .parse::<u16>()
                .unwrap_or_else(|error| panic!("activity: {error}"));
            let crc = u32::from_str_radix(words.next()?, 16)
                .unwrap_or_else(|error| panic!("CRC: {error}"));
            (activity & 0x200 != 0 && frames.contains(step)).then_some(crc)
        })
        .collect()
}
struct Bridge {
    socket: TcpStream,
    net: LoopbackNet,
    connection: Connection,
    output: Vec<u8>,
    forwarded: usize,
    wire: Vec<u8>,
    checksums: Vec<(u32, u32)>,
    inputs: usize,
}
impl Bridge {
    fn new(socket: TcpStream, net: LoopbackNet, connection: Connection) -> Result<Self> {
        socket.set_nonblocking(true)?;
        socket.set_nodelay(true)?;
        Ok(Self {
            socket,
            net,
            connection,
            output: Vec::new(),
            forwarded: 0,
            wire: Vec::new(),
            checksums: Vec::new(),
            inputs: 0,
        })
    }
    fn observe(&mut self, bytes: &[u8]) {
        self.wire.extend_from_slice(bytes);
        loop {
            match Packet::decode(&self.wire) {
                Ok((packet, count)) => {
                    match packet {
                        Packet::Checksum { frame, checksum } => {
                            self.checksums.push((frame, checksum))
                        }
                        Packet::BattleInput(_) => self.inputs += 1,
                        _ => {}
                    }
                    self.wire.drain(..count);
                }
                Err(DecodeError::Incomplete) => break,
                Err(error) => panic!("invalid live packet: {error:?}"),
            }
        }
    }
    fn poll(&mut self) -> Result<()> {
        let mut bytes = [0; 8192];
        match self.socket.read(&mut bytes) {
            Ok(count) if count != 0 => {
                assert_eq!(self.net.send(self.connection, &bytes[..count])?, count);
                self.forwarded += count;
                self.observe(&bytes[..count]);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        match self.net.recv(self.connection, &mut bytes) {
            Ok(count) => self.output.extend_from_slice(&bytes[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        if !self.output.is_empty() {
            match self.socket.write(&self.output) {
                Ok(count) => {
                    self.output.drain(..count);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
fn join_peers(
    listener: TcpListener,
    port: u16,
    deadline: Instant,
    ready_file: &Path,
) -> Result<[TcpStream; 2]> {
    let mut ready_file = fs::File::open(ready_file)?;
    let mut readiness = String::new();
    listener.set_nonblocking(true)?;
    let mut sockets = [None, None];
    while sockets.iter().any(Option::is_none) {
        if sockets[0].is_none() {
            match listener.accept() {
                Ok((socket, _)) => sockets[0] = Some(socket),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
            }
        }
        ready_file.read_to_string(&mut readiness)?;
        if sockets[1].is_none() && readiness.contains("netplay fixture listener ready") {
            sockets[1] = Some(TcpStream::connect(("127.0.0.1", port))?);
        }
        assert!(Instant::now() < deadline, "peer startup deadline");
        std::thread::yield_now();
    }
    Ok(sockets.map(|socket| socket.unwrap_or_else(|| panic!("both peer sockets connected"))))
}
#[test]
fn two_battle_peers_match_melee_input_at_every_checksum_interval() -> Result<()> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temporary = manifest.join("../tmp");
    fs::create_dir_all(&temporary)?;
    let root = tempfile::Builder::new()
        .prefix("netplay-loopback-")
        .tempdir_in(temporary)?;
    let root_path = root.path().canonicalize()?;
    eprintln!("live loopback artifacts {}", root_path.display());
    let source = manifest.join("../sc2/content");
    zip::ZipArchive::new(fs::File::open(
        source.join("packages/uqm-0.8.0-content.uqm"),
    )?)?
    .extract(root_path.join("content"))?;
    fs::copy(source.join("version"), root_path.join("content/version"))?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let reserved = TcpListener::bind("127.0.0.1:0")?;
    let port = reserved.local_addr()?.port();
    let mut a = start_replay(&root_path, 0, listener.local_addr()?.port())?;
    drop(reserved);
    let mut b = start_replay(&root_path, 1, port)?;
    let deadline = Instant::now() + Duration::from_secs(420);
    let [a_socket, b_socket] = join_peers(listener, port, deadline, &root_path.join("1/game.log"))
        .map_err(|error| format!("join peers: {error}"))?;
    let (mut a_net, mut b_net) = LoopbackNet::pair();
    let server = a_net.listen("battle")?;
    let client = b_net.connect("loopback", "battle")?;
    let mut bridges = [
        Bridge::new(a_socket, a_net, server)?,
        Bridge::new(b_socket, b_net, client)?,
    ];
    loop {
        for bridge in &mut bridges {
            bridge.poll().map_err(|error| {
                format!(
                    "forward peer: {error}; logs: {}\n{}",
                    fs::read_to_string(root_path.join("0/game.log"))
                        .unwrap_or_else(|error| panic!("peer log: {error}")),
                    fs::read_to_string(root_path.join("1/game.log"))
                        .unwrap_or_else(|error| panic!("peer log: {error}"))
                )
            })?;
        }
        let statuses = [a.0.try_wait()?, b.0.try_wait()?];
        for (peer, status) in statuses.iter().enumerate() {
            if let Some(status) = status {
                assert!(
                    status.success(),
                    "peer {peer}: {}",
                    fs::read_to_string(root_path.join(peer.to_string()).join("game.log"))?
                );
            }
        }
        if statuses.iter().all(Option::is_some) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "battle replay exceeded scenario budget; artifacts {}",
            root_path.display()
        );
        std::thread::yield_now();
    }
    let expected = battle_checksums(
        include_str!("golden/parity/melee-input/parity.trace"),
        include_str!("golden/parity/melee-input/parity.fields"),
    );
    assert!(!expected.is_empty());
    assert_eq!(
        bridges[0].checksums, bridges[1].checksums,
        "wire checksums agree every interval"
    );
    for bridge in bridges {
        assert!(
            bridge.inputs >= expected.len(),
            "live inputs for every battle frame"
        );
        assert_eq!(bridge.checksums.len(), expected.len() + 1);
        for (frame, (number, _)) in bridge.checksums.iter().enumerate() {
            assert_eq!(*number, frame as u32);
        }
        // Wire checksums precede input and simulation. Golden battle checkpoints
        // follow simulation. The final end-handshake call records the last state
        // without advancing another simulated frame.
        assert_eq!(
            bridge
                .checksums
                .iter()
                .skip(1)
                .map(|(_, crc)| *crc)
                .collect::<Vec<_>>(),
            expected
        );
    }
    for peer in 0..2 {
        let directory = root_path.join(peer.to_string());
        let summary: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.join("summary.json"))?)?;
        assert_eq!(summary["completed"], true);
        let actual = battle_checksums(
            &fs::read_to_string(directory.join("automation/parity.trace"))?,
            &fs::read_to_string(directory.join("automation/parity.fields"))?,
        );
        assert_eq!(actual, expected, "peer {peer} battle checksum stream");
    }
    Ok(())
}
