use super::abi::*;
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
    pub battle_state: *mut battlestate_struct,
    pub get_melee_state: *mut getmelee_struct,
    pub end_frame_count: BattleFrameCounter,
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

#[cfg(test)]
mod tests {
    use super::*;
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
