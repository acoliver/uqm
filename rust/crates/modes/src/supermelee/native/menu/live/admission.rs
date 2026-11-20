//! The live menu lends setup effects only for the current admitted input call.
use crate::engine::battle::net::NetState;
use crate::engine::world::activity::activity_flags::IN_BATTLE;
use crate::engine::{
    battle::Cx,
    input::Frame,
    observe::{Injected, InputSetup, InputView, Io, Observer, PeerSetup},
};
use crate::supermelee::session::MeleeSession;

struct Setup<'s, 'c, 'w> {
    state: &'s mut MeleeSession,
    cx: &'c mut Cx<'w>,
}

impl InputSetup for Setup<'_, '_, '_> {
    fn configure_fleet(&mut self, ships: [[u8; 7]; 2], cyborg: bool) -> Result<(), String> {
        self.state.configure_fleets(
            &mut self.cx.kernel.player_control,
            self.cx.world.activity() & IN_BATTLE != 0,
            ships,
            cyborg,
        )
    }

    fn connect_peer(
        &mut self,
        remote_player: u16,
        server: bool,
        port: u16,
        observer: &mut dyn Observer,
    ) -> Result<Option<u16>, String> {
        if self.state.melee_started != 0 {
            return Err("cannot connect the netplay fixture during battle".into());
        }
        let side = usize::from(remote_player);
        self.state.netplay.configure_peer(side, server, port);
        let session = self.state.netplay.clone();
        let state = &mut *self.state;
        let cx = &mut *self.cx;
        let io = &mut Io { observer };
        if !session.open_player_network_connection(remote_player, &mut |event| {
            super::changes::apply_protocol_event(state, cx, io, event)
        }) {
            return Err("could not open netplay fixture endpoint".into());
        }
        let socket = session
            .connection(side)
            .unwrap_or_else(|| panic!("opened connection"))
            .socket;
        if server {
            let socket = socket.unwrap_or_else(|| panic!("opened endpoint has no socket"));
            session
                .local_port(socket)
                .map(Some)
                .map_err(|error| format!("fixture port: {error}"))
        } else {
            Ok(None)
        }
    }

    fn peer_setup(&self, remote_player: u16) -> Option<PeerSetup> {
        let side = usize::from(remote_player);
        let connection = self.state.netplay.connection(side)?;
        Some(PeerSetup {
            ready: connection.state == NetState::InSetup
                && self.state.melee_setup.sent[side].is_synchronized(),
            disconnected: connection.flags.disconnected,
        })
    }

    fn poll_peer(&mut self, observer: &mut dyn Observer) {
        let session = self.state.netplay.clone();
        let state = &mut *self.state;
        let cx = &mut *self.cx;
        let io = &mut Io { observer };
        session.net_input_with(1, &mut |event| {
            super::changes::apply_protocol_event(state, cx, io, event)
        });
    }

    fn with_context(&mut self, operation: &mut dyn FnMut(&mut Cx<'_>)) {
        operation(self.cx);
    }
}

pub(crate) fn observe(
    state: &mut MeleeSession,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    view: InputView,
) -> Injected {
    io.observer
        .input_admitted_with_setup(&view, &mut Setup { state, cx })
}

pub(crate) struct SessionFrame<'a, F> {
    state: &'a mut MeleeSession,
    step: F,
}

impl<'a, F> SessionFrame<'a, F> {
    pub(crate) fn new(state: &'a mut MeleeSession, step: F) -> Self {
        Self { state, step }
    }
}

impl<F> Frame for SessionFrame<'_, F>
where
    F: FnMut(&mut MeleeSession, &mut Cx<'_>, &mut Io<'_>) -> bool,
{
    fn admit(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, view: InputView) -> Injected {
        observe(self.state, cx, io, view)
    }

    fn frame(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) -> bool {
        (self.step)(self.state, cx, io)
    }
}
