use super::{
    abi::*,
    checkbuf::ChecksumBuffer,
    netconnection::{NetConnection, Options},
};
use std::cell::{Cell, Ref, RefCell, RefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPhase {
    Menu,
    Confirm,
    Connecting,
    Editing,
    Loading,
    Building,
}

#[repr(C)]
#[derive(Debug)]
pub struct MeleeState {
    pub phase: MenuPhase,
    pub initialized: BOOLEAN,
    pub melee_started: BOOLEAN,
    pub melee_option: MELEE_OPTIONS,
    pub side: COUNT,
    pub row: COUNT,
    pub col: COUNT,
    pub melee_setup: crate::supermelee::setup::team::MeleeSetup,
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

#[derive(Debug, Default)]
pub(super) struct Selection {
    pub mask: u16,
    pub available: [[bool; 14]; 2],
    pub choices: [Option<u16>; 2],
}
pub struct SessionCore {
    controller: Option<crate::engine::input::InputSession>,
    pub options: Options,
    pub(super) replay_remote_player: Option<usize>,
    pub(super) transport: RefCell<Box<dyn uqm_engine::host::Net>>,
    pub(super) frame_count: Cell<u32>,
    pub(super) connections: [RefCell<Option<NetConnection>>; 2],
    pub(super) input: RefCell<super::netinput::InputBuffers>,
    pub(super) checksums: RefCell<ChecksumBuffer>,
    pub(super) selection: RefCell<Selection>,
}
impl SessionCore {
    pub(crate) fn controller(&self) -> &crate::engine::input::InputSession {
        self.controller
            .as_ref()
            .unwrap_or_else(|| panic!("native protocol controller is not installed"))
    }
    fn new(
        transport: Box<dyn uqm_engine::host::Net>,
        controller: Option<crate::engine::input::InputSession>,
    ) -> Self {
        Self {
            controller,
            options: Options::default(),
            replay_remote_player: None,
            transport: RefCell::new(transport),
            frame_count: Cell::new(0),
            connections: std::array::from_fn(|_| RefCell::new(None)),
            input: RefCell::new(super::netinput::InputBuffers::default()),
            checksums: RefCell::new(ChecksumBuffer::default()),
            selection: RefCell::new(Selection::default()),
        }
    }
    pub fn connection(&self, player: usize) -> Option<Ref<'_, NetConnection>> {
        Ref::filter_map(self.connections[player].borrow(), Option::as_ref).ok()
    }
    pub fn connection_mut(&self, player: usize) -> Option<RefMut<'_, NetConnection>> {
        RefMut::filter_map(self.connections[player].borrow_mut(), Option::as_mut).ok()
    }
    pub fn local_port(&self, socket: uqm_engine::host::Connection) -> std::io::Result<u16> {
        self.transport.borrow().local_port(socket)
    }
    pub fn select_remote(&self, side: usize, choice: u16) -> bool {
        let mut selection = self.selection.borrow_mut();
        if selection.mask & (1 << side) == 0 || selection.choices[side].is_some() {
            return false;
        }
        let valid = if choice == u16::MAX {
            selection.available[side].iter().any(|available| *available)
        } else {
            selection.available[side]
                .get(usize::from(choice))
                .copied()
                .unwrap_or(false)
        };
        if valid {
            selection.choices[side] = Some(choice);
        }
        valid
    }
    pub(crate) fn replay_wait(&self, poll: impl FnMut() -> bool) -> bool {
        poll_replay_wait(self.replay_remote_player.is_some(), poll, || {
            crate::threading::pacing::yield_input_loop(std::thread::yield_now);
        })
    }
}
impl Default for SessionCore {
    fn default() -> Self {
        Self::new(uqm_engine::host::native_net(), None)
    }
}
impl Drop for SessionCore {
    fn drop(&mut self) {
        for connection in &mut self.connections {
            if let Some(socket) = connection
                .get_mut()
                .as_mut()
                .and_then(|conn| conn.socket.take())
            {
                self.transport.get_mut().close(socket);
            }
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct NetplaySession(std::rc::Rc<SessionCore>);
impl std::ops::Deref for NetplaySession {
    type Target = SessionCore;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl NetplaySession {
    pub fn with_input(input: crate::engine::input::InputSession) -> Self {
        Self(std::rc::Rc::new(SessionCore::new(
            uqm_engine::host::native_net(),
            Some(input),
        )))
    }
    pub fn battle_net(&self) -> std::rc::Rc<dyn crate::engine::battle::net::BattleNet> {
        self.0.clone()
    }
    pub fn options_mut(&mut self) -> &mut Options {
        &mut std::rc::Rc::get_mut(&mut self.0)
            .unwrap_or_else(|| panic!("cannot edit options while battle owns the session"))
            .options
    }
    pub fn with_net(net: impl uqm_engine::host::Net + 'static) -> Self {
        Self(std::rc::Rc::new(SessionCore::new(Box::new(net), None)))
    }
    pub(crate) fn configure_peer(&mut self, side: usize, server: bool, port: u16) {
        let core = std::rc::Rc::get_mut(&mut self.0)
            .unwrap_or_else(|| panic!("menu exclusively owns the session"));
        core.options.peer[side] = super::netconnection::PeerOptions {
            server,
            host: "127.0.0.1".into(),
            port: port.to_string(),
        };
        core.replay_remote_player = Some(side);
        core.options.input_delay = 0;
        core.set_battle_input_delay(0);
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
    fn protocol_session_retains_the_supplied_input_and_configuration() {
        let input = crate::engine::input::InputSession::default();
        let session = NetplaySession::with_input(input.clone());
        input.set_player_control(1, 5);
        input
            .configuration
            .update_preferences(|options| options.menu = 2);
        assert_eq!(session.controller().player_controls()[1], 5);
        assert_eq!(session.controller().preferences().menu, 2);
        assert_eq!(session.controller().immediate_ptr(), input.immediate_ptr());
    }
    #[test]
    fn fixture_server_retains_its_bound_port_until_connection() {
        use uqm_engine::host::Net;
        let mut net = uqm_engine::network::NativeNet::default();
        let endpoint = net.listen("0").unwrap();
        let session = NetplaySession::with_net(net);
        let port = session.local_port(endpoint).unwrap();
        assert_ne!(port, 0);
        assert!(std::net::TcpListener::bind(("0.0.0.0", port)).is_err());
        let peer = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        assert_eq!(peer.peer_addr().unwrap().port(), port);
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
            || yields += 1
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
        session.options_mut().input_delay = 3;
        let second = session.battle_net();
        assert!(std::ptr::addr_eq(identity, std::rc::Rc::as_ptr(&second)));
        assert_eq!(second.frame(), 19);
        assert_eq!(session.options.input_delay, 3);
    }
    #[test]
    fn configured_endpoint_text_remains_owned_by_the_session() {
        let mut session =
            NetplaySession::with_net(uqm_engine::network::loopback::LoopbackNet::default());
        session.configure_peer(1, false, 45678);
        assert!(!session.options.peer[1].server);
        assert_eq!(session.options.input_delay, 0);
        assert_eq!(session.get_battle_input_delay(), 0);
        assert_eq!(session.options.peer[1].port, "45678");
    }
}
