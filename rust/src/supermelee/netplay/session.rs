use super::abi::*;
use std::{
    cell::{Cell, RefCell},
    ptr,
};

pub struct SessionCore {
    pub options: NetplayOptions,
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
            .expect("cannot edit options while battle owns the session")
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
