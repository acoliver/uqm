use super::abi::*;
use crate::engine::battle::schema::getmelee_struct;
use std::{
    cell::{Cell, RefCell},
    ptr,
};

#[repr(C)]
#[derive(Debug)]
pub struct MeleeState {
    pub input_func:
        ::std::option::Option<unsafe extern "C" fn(_input_state: *mut MeleeState) -> BOOLEAN>,
    pub initialized: BOOLEAN,
    pub melee_started: BOOLEAN,
    pub melee_option: MELEE_OPTIONS,
    pub side: COUNT,
    pub row: COUNT,
    pub col: COUNT,
    pub melee_setup: *mut MeleeSetup,
    pub load: melee_load_state,
    pub current_ship: MeleeShip,
    pub cur_index: COUNT,
    pub build_pick_confirmed: BOOLEAN,
    pub random_context: *mut RandomContext,
    pub last_input_time: TimeCount,
    pub h_music: MUSIC_REF,
    pub netplay: NetplaySession,
    pub(crate) catalog: Vec<crate::ships::master_native::ShipChoice>,
}

pub struct SessionCore {
    pub options: NetplayOptions,
    peer_ports: [Option<std::ffi::CString>; 2],
    pub(super) replay_remote_player: Option<usize>,
    pub(super) transport: RefCell<Box<dyn uqm_engine::host::Net>>,
    pub(super) frame_count: Cell<u32>,
    pub(super) connections: Cell<[*mut NetConnection; 2]>,
    pub(super) input: RefCell<super::netinput::InputBuffers>,
    pub(super) checksums: RefCell<ChecksumBuffer>,
}

impl SessionCore {
    fn new(transport: Box<dyn uqm_engine::host::Net>) -> Self {
        Self {
            options: super::netoptions::defaults(),
            peer_ports: [None, None],
            replay_remote_player: None,
            transport: RefCell::new(transport),
            frame_count: Cell::new(0),
            connections: Cell::new([ptr::null_mut(); 2]),
            input: RefCell::new(super::netinput::InputBuffers::default()),
            checksums: RefCell::new(ChecksumBuffer {
                checksums: ptr::null_mut(),
                maxSize: 0,
                interval: 0,
            }),
        }
    }
}

impl Default for SessionCore {
    fn default() -> Self {
        Self::new(uqm_engine::host::native_net())
    }
}

impl Drop for SessionCore {
    fn drop(&mut self) {
        // SAFETY: the last session handle owns its connection records and buffers;
        // no battle or network callback can retain a strong handle during teardown.
        unsafe {
            self.close_all_connections();
            self.uninit_battle_input_buffers();
            super::checkbuf::ChecksumBuffer_uninit(self.checksums.get_mut());
        }
    }
}

#[derive(Debug, Default)]
pub struct NetplaySession(std::rc::Rc<SessionCore>);

impl std::ops::Deref for NetplaySession {
    type Target = SessionCore;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl NetplaySession {
    pub fn battle_net(&self) -> std::rc::Rc<dyn crate::engine::battle::net::BattleNet> {
        self.0.clone()
    }
    pub fn options_mut(&mut self) -> &mut NetplayOptions {
        &mut std::rc::Rc::get_mut(&mut self.0)
            .unwrap_or_else(|| panic!("cannot edit options while battle owns the session"))
            .options
    }
}

impl std::fmt::Debug for SessionCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionCore")
            .field("options", &self.options)
            .field("connections", &self.connections)
            .finish_non_exhaustive()
    }
}
impl NetplaySession {
    pub fn with_net(net: impl uqm_engine::host::Net + 'static) -> Self {
        Self(std::rc::Rc::new(SessionCore::new(Box::new(net))))
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BattleStateData {
    pub release_function: NetConnectionStateData_ReleaseFunction,
    pub melee_state: *mut melee_state,
    pub battle_active: bool,
    pub get_melee_state: *mut getmelee_struct,
    pub end_frame_count: BattleFrameCounter,
}

impl SessionCore {
    pub(crate) fn local_port(&self, socket: uqm_engine::host::Connection) -> std::io::Result<u16> {
        self.transport.borrow().local_port(socket)
    }

    #[cfg(not(test))]
    pub(crate) fn replay_confirmation(&self, mut ready: impl FnMut() -> isize) -> isize {
        let mut count = ready();
        self.replay_wait(|| {
            if count != 1 {
                return false;
            }
            <SessionCore as crate::engine::battle::net::BattleNet>::poll(self);
            count = ready();
            count == 1
        });
        count
    }

    // Network readiness polls must not consume the replay's next gameplay input.
    pub(crate) fn replay_wait(&self, poll: impl FnMut() -> bool) -> bool {
        poll_replay_wait(self.replay_remote_player.is_some(), poll, || {
            crate::threading::pacing::yield_input_loop(std::thread::yield_now);
        })
    }
}

impl NetplaySession {
    pub(crate) fn configure_peer(&mut self, side: usize, server: bool, port: u16) {
        let core = std::rc::Rc::get_mut(&mut self.0)
            .unwrap_or_else(|| panic!("menu exclusively owns the session"));
        core.peer_ports[side] = Some(
            std::ffi::CString::new(port.to_string())
                .unwrap_or_else(|error| panic!("numeric port: {error}")),
        );
        core.options.peer[side].isServer = server;
        core.options.peer[side].host = c"127.0.0.1".as_ptr();
        core.options.peer[side].port = core.peer_ports[side]
            .as_ref()
            .unwrap_or_else(|| panic!("configured port missing"))
            .as_ptr();
        core.replay_remote_player = Some(side);
        core.options.inputDelay = 0;
        core.set_battle_input_delay(0);
    }
}

fn poll_replay_wait(
    replay: bool,
    mut poll: impl FnMut() -> bool,
    mut yield_wait: impl FnMut(),
) -> bool {
    if !replay {
        return false;
    }
    while poll() {
        yield_wait();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_server_retains_its_bound_port_until_connection() {
        use uqm_engine::host::Net;
        let mut net = uqm_engine::network::NativeNet::default();
        let endpoint = net
            .listen("0")
            .unwrap_or_else(|error| panic!("listen: {error}"));
        let session = NetplaySession::with_net(net);
        let port = session
            .local_port(endpoint)
            .unwrap_or_else(|error| panic!("port: {error}"));
        assert_ne!(port, 0);
        assert!(std::net::TcpListener::bind(("0.0.0.0", port)).is_err());
        let peer = std::net::TcpStream::connect(("127.0.0.1", port))
            .unwrap_or_else(|error| panic!("connect: {error}"));
        assert_eq!(
            peer.peer_addr()
                .unwrap_or_else(|error| panic!("address: {error}"))
                .port(),
            port
        );
    }

    #[test]
    fn replay_protocol_waits_poll_without_admitting_input() {
        let mut polls = 0;
        let mut yields = 0;
        assert!(poll_replay_wait(
            true,
            || {
                polls += 1;
                polls <= 10_001
            },
            || yields += 1,
        ));
        assert_eq!(polls, 10_002);
        assert_eq!(yields, 10_001);
        assert!(!poll_replay_wait(
            false,
            || panic!("native wait polled"),
            || panic!("native wait yielded")
        ));
    }

    #[test]
    fn consecutive_battles_borrow_the_same_menu_session() {
        let mut session = NetplaySession::default();
        let first = session.battle_net();
        first.set_frame(19);
        let identity = std::rc::Rc::as_ptr(&first);
        drop(first);
        session.options_mut().inputDelay = 3;
        let second = session.battle_net();
        assert!(std::ptr::addr_eq(identity, std::rc::Rc::as_ptr(&second)));
        assert_eq!(second.frame(), 19);
        assert_eq!(session.options.inputDelay, 3);
    }

    #[test]
    fn configured_endpoint_text_remains_owned_by_the_session() {
        let mut session =
            NetplaySession::with_net(uqm_engine::network::loopback::LoopbackNet::default());
        session.configure_peer(1, false, 45678);
        assert!(!session.options.peer[1].isServer);
        assert_eq!(session.options.inputDelay, 0);
        assert_eq!(session.get_battle_input_delay(), 0);
        assert_eq!(
            session.peer_ports[1]
                .as_ref()
                .unwrap_or_else(|| panic!("configured port missing"))
                .to_bytes(),
            b"45678"
        );
    }
}
