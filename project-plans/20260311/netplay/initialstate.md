# Netplay initial state

## Scope and boundary of this document

This document describes the currently active C implementation of the SuperMelee netplay subsystem under `sc2/src/uqm/supermelee/netplay/`, plus its immediate integration points into the surrounding SuperMelee and lower-level network/callback layers.

The subsystem is conditionally compiled behind `NETPLAY` in `sc2/src/uqm/supermelee/netplay/netplay.h:19-55`. It provides peer-to-peer network play for SuperMelee battles using direct TCP connections created through the lower-level `libs/network` stack. The active implementation covers:

- network connection establishment and teardown,
- connection state tracking (`NetState`),
- packet definition, parsing, queuing, and sending,
- protocol version and minimum-UQM-version validation,
- setup-screen fleet and team-name synchronization,
- confirmation, ready, and reset sub-protocols,
- battle-time ship-selection and input exchange,
- battle input delay negotiation and buffering,
- optional checksum-based sync verification, and
- user-facing notification hooks back into SuperMelee UI flow.

This document does not redefine:

- SuperMelee setup/menu ownership outside the netplay boundary,
- battle simulation rules,
- ship behavior,
- generic socket management or callback scheduling semantics outside the netplay-specific way they are used.

Those are dependencies and integration boundaries, not netplay-owned policy.

## Verified implementation footprint

The active netplay C implementation is concentrated in the following files listed by `sc2/src/uqm/supermelee/netplay/Makeinfo:2-3` and present in the source tree:

- transport/state/core: `netconnection.c`, `netconnection.h`, `nc_connect.ci`, `netstate.c`, `netstate.h`, `netmelee.c`, `netmelee.h`, `netmisc.c`, `netmisc.h`, `netoptions.c`, `netoptions.h`, `netplay.h`
- receive/send/queue: `netrcv.c`, `netrcv.h`, `netsend.c`, `netsend.h`, `packetq.c`, `packetq.h`
- packet definitions and behavior: `packet.c`, `packet.h`, `packethandlers.c`, `packethandlers.h`, `packetsenders.c`, `packetsenders.h`
- notifications: `notify.c`, `notify.h`, `notifyall.c`, `notifyall.h`
- battle input and sync verification: `netinput.c`, `netinput.h`, `checkbuf.c`, `checkbuf.h`, `checksum.c`, `checksum.h`, `crc.c`, `crc.h`
- protocol submodules: `proto/npconfirm.c`, `proto/npconfirm.h`, `proto/ready.c`, `proto/ready.h`, `proto/reset.c`, `proto/reset.h`

The subsystem is integrated into the surrounding game via `sc2/src/uqm/supermelee/melee.c`, `sc2/src/uqm/supermelee/pickmele.c`, `sc2/src/uqm/battle.c`, and `sc2/src/uqm/tactrans.c`.

## Current compilation and protocol constants

`sc2/src/uqm/supermelee/netplay/netplay.h:24-55` defines the active compile-time protocol constants and features:

- protocol version: major `0`, minor `4`
- minimum accepted remote UQM version: `0.6.9`
- `NETPLAY_STATISTICS` enabled
- `NETPLAY_CHECKSUM` enabled
- checksum interval: `1` frame
- read buffer size: `2048` bytes
- connect timeout: `2000` ms
- retry delay: `2000` ms
- listen backlog: `2`
- transport mode can be `NETPLAY_FULL` or `NETPLAY_IPV4`; `nc_connect.ci` chooses `PF_inet` only when built as `NETPLAY_IPV4`, otherwise `PF_unspec`

Default runtime options live in `sc2/src/uqm/supermelee/netplay/netoptions.c:19-37`:

- meta server: `uqm.stack.nl:21836`
- peer defaults for both players: host `localhost`, port `21837`
- both peers default to `isServer = true`
- default requested input delay: `2`

The meta-server fields exist in `NetplayOptions`, but the code read for this subsystem documentation does not show SuperMelee netplay actually using them during connection setup. Active peer-to-peer setup uses `netplayOptions.peer[player]` through `openPlayerNetworkConnection()` in `netmelee.c:378-392`.

## Current ownership model and boundaries

### What netplay owns

The current netplay subsystem owns:

- `NetConnection` lifecycle and connection-local state (`netconnection.c`, `netconnection.h`)
- the `NetState` state machine (`netstate.h:25-42`)
- wire packet schema and packet handlers (`packet.h`, `packethandlers.c`)
- setup synchronization messages for fleet/team-name changes (`notify.c`, `notifyall.c`, packet handlers in `packethandlers.c`)
- handshake/confirmation behavior (`proto/npconfirm.c`, handshake packet handlers in `packethandlers.c`)
- generic ready synchronization behavior (`proto/ready.c`)
- reset/abort behavior (`proto/reset.c`, abort/reset packet handlers)
- input delay negotiation, battle input buffering, and network battle input delivery (`netinput.c`, `netmelee.c`, `packethandlers.c`)
- checksum buffering and state CRC verification (`checkbuf.c`, `checksum.c`, `crc.c`, battle hooks in `battle.c`)
- transport polling orchestration over the lower network/callback libraries (`netmelee.c:82-112`)

### What SuperMelee owns at the boundary

SuperMelee still owns UI flow and gameplay orchestration around netplay. Examples:

- `StartMeleeButtonPressed()` in `sc2/src/uqm/supermelee/melee.c:1491-1581` enforces menu-level netplay preconditions before entering confirmation flow.
- `DoConfirmSettings()` in `melee.c:1279-1396` drives the start-of-match confirmation loop, sends preferred input delay, triggers RNG seed exchange, and then starts battle.
- `Melee_bootstrapSyncTeam()` plus `Melee_LocalChange_*`/`Melee_RemoteChange_*` in `melee.c:2374-2635` own the actual team-state mutation rules and turn-based conflict resolution for setup edits.
- `updateMeleeSelection()` and `reportShipSelected()` in `pickmele.c:907-947` own the ship-pick UI state that netplay drives during battle selection.
- UI/player feedback functions like `connectedFeedback`, `abortFeedback`, `resetFeedback`, `errorFeedback`, and `closeFeedback` are defined in `melee.c:2144-2263` and invoked by netplay callbacks.

### What battle owns at the boundary

Battle owns simulation and frame progression, but netplay hooks into it:

- `battle.c:268-304` computes/sends local checksums and verifies delayed checksums after `ProcessInput()`.
- `battle.c:437-466` initializes network input buffers and checksum buffers, resets frame count, and negotiates transition into `NetState_inBattle`.
- `tactrans.c:152-227` drives the end-of-battle frame-count synchronization protocol using netplay ready states and `FrameCount` packets.

### What lower-level libs own

The lower-level networking and callback libraries own sockets, descriptor registration, callback scheduling, timers, and event multiplexing:

- `libs/network/netmanager/*` owns `NetDescriptor`, callback registration, and `select()`/Winsock event dispatch.
- `libs/network/connect/*` is pulled in via `libs/net.h` and is used by `listenPort()` / `connectHostByName()` from `nc_connect.ci`.
- `libs/callback/callback.c` owns deferred callback queue processing.
- `libs/callback/alarm.c` and `libs/callback/async.c` own timed alarms and combined async processing.

Netplay is written around those facilities rather than implementing its own socket loop or timer queue.

## Current connection lifecycle

### Global connection registry

`sc2/src/uqm/supermelee/netplay/netmelee.c:43-80` maintains the process-global connection registry:

- `NetConnection *netConnections[NUM_PLAYERS]`
- `size_t numNetConnections`
- helpers `addNetConnection`, `removeNetConnection`, `closeAllConnections`, `closeDisconnectedConnections`, `forEachConnectedPlayer`, `getNumNetConnections`

The subsystem assumes one `NetConnection` per player slot. Most loops iterate `player = 0..NUM_PLAYERS-1` and skip `NULL` or disconnected slots.

### Opening a connection

`openPlayerNetworkConnection()` in `netmelee.c:378-392` creates a `NetConnection` with:

- player number,
- `netplayOptions.peer[player]`,
- connect callback `NetMelee_connectCallback`,
- close callback `NetMelee_closeCallback`,
- error callback `NetMelee_errorCallback`,
- delete callback `deleteConnectionCallback`, and
- an `extra` pointer, which is initially the `MELEE_STATE *`.

`NetConnection_open()` in `netconnection.c:46-156` allocates the connection, initializes state flags, creates a packet queue, allocates the read buffer, and immediately calls `NetConnection_go()`.

### Server/client branching

`NetConnection_go()` is defined in `nc_connect.ci:70-100` and chooses:

- `NetConnection_serverGo()` if `options->isServer` is true
- `NetConnection_clientGo()` otherwise

Server mode (`nc_connect.ci:102-128`):

- requires current state `NetState_unconnected`
- sets state to `NetState_connecting`
- builds `ListenFlags`
- calls `listenPort(port, IPProto_tcp, &listenFlags, ...)`

Client mode (`nc_connect.ci:130-159`):

- requires current state `NetState_unconnected`
- sets state to `NetState_connecting`
- builds `ConnectFlags`
- sets timeout and retry delay from compile-time constants
- calls `connectHostByName(host, port, IPProto_tcp, &connectFlags, ...)`

In both cases a `ConnectStateData` wrapper is stored in `conn->stateData` so the in-progress listen/connect object can later be closed or freed.

### Transition to connected

On successful accept/connect:

- server callback: `NetConnection_connectedServerCallback()` in `nc_connect.ci:161-186`
- client callback: `NetConnection_connectedClientCallback()` in `nc_connect.ci:188-212`

Both callbacks:

- store the accepted/connected `NetDescriptor *` in `conn->nd`
- attach `conn` as descriptor extra data
- call `Socket_setInteractive()` on the underlying socket
- assign the `discriminant` flag differently:
  - server side gets `true`
  - client side gets `false`
- call `NetConnection_connected()`

`NetConnection_connected()` in `nc_connect.ci:214-231` then:

- sets `stateFlags.connected = true`
- sets state to `NetState_init`
- destroys the temporary connect/listen state data
- installs descriptor callbacks:
  - read callback `dataReadyCallback`
  - close callback `closeCallback`
- invokes the connection's higher-level connect callback (`NetMelee_connectCallback`)

### Initial netplay setup after TCP connect

`NetMelee_connectCallback()` in `netmisc.c:74-94` performs the first netplay-layer transition:

- replaces the original `extra` pointer (`MELEE_STATE *`) with a new `BattleStateData` state object stored in `conn->stateData`
- clears `extra`
- resets setup synchronization sent-state with `MeleeSetup_resetSentTeams()`
- queues an `Init` packet with `sendInit(conn)`
- calls `Netplay_localReady(conn, NetMelee_enterState_inSetup, NULL, false)`

This means a freshly connected connection enters `NetState_init` and waits for the remote side's `Init` packet/ready transition before entering setup synchronization.

### Entering setup state

`PacketHandler_Init()` in `packethandlers.c:55-118` validates the remote protocol/UQM version and, on success, calls `Netplay_remoteReady(conn)`.

When both sides become ready for this init phase, the callback passed earlier fires: `NetMelee_enterState_inSetup()` in `netmisc.c:110-129`.

That callback:

- sets state to `NetState_inSetup`
- emits UI feedback (`connectedFeedback`)
- sends the entire current team for this player via `Melee_bootstrapSyncTeam()`
- flushes packet queues immediately

So current behavior is: TCP connect → `NetState_init` → exchange/validate `Init` → ready rendezvous → `NetState_inSetup` → bootstrap team sync.

### Error and close handling during connection establishment

The listen/connect error callbacks in `nc_connect.ci:233-291`:

- mark the connection disconnected
- close the in-progress listen/connect state object
- set state back to `NetState_unconnected`
- free state data
- invoke the error callback with `NetConnectionError`
- call `NetConnection_close(conn)`

After a fully established connection exists, runtime close handling goes through:

- `closeCallback()` in `netconnection.c:212-219` when the `NetDescriptor` closes,
- which clears `conn->nd` and calls `NetConnection_doClose()`.

`NetConnection_doClose()` in `netconnection.c:195-210`:

- marks the connection disconnected,
- invokes the higher-level close callback while state/stateData are still present,
- then sets state to `NetState_unconnected`.

`NetMelee_closeCallback()` maps that to SuperMelee UI feedback via `closeFeedback()` (`netmisc.c:96-99`).

### Deletion and registry cleanup

`NetConnection_close()` in `netconnection.c:221-236`:

- detaches the descriptor close callback,
- closes the descriptor,
- forces local close handling if needed,
- then deletes the connection object.

Deletion triggers `deleteConnectionCallback()` from `netmelee.c:368-376`, which removes the pointer from `netConnections[player]` and decrements `numNetConnections`.

## Current `NetState` state machine

`sc2/src/uqm/supermelee/netplay/netstate.h:25-42` defines the current connection state enum:

- `NetState_unconnected`
- `NetState_connecting`
- `NetState_init`
- `NetState_inSetup`
- `NetState_preBattle`
- `NetState_interBattle`
- `NetState_selectShip`
- `NetState_inBattle`
- `NetState_endingBattle`
- `NetState_endingBattle2`

`netstate.c:24-37` mirrors those names for debugging.

### Meaning of the states in the current code

- `unconnected`: no active connection
- `connecting`: asynchronous listen/connect in progress (`nc_connect.ci`)
- `init`: TCP connected, protocol/version init exchange in progress (`NetMelee_connectCallback`, `PacketHandler_Init`)
- `inSetup`: SuperMelee setup screen sync active; fleet/team-name updates and handshake confirmation are valid here (`notify.c`, `packethandlers.c`, `handshakeMeaningful()` in `netmisc.h:56-59`)
- `preBattle`: start-of-match negotiation after setup confirmation; RNG seed and input delay packets are exchanged here (`notify.c:82-95`, `PacketHandler_SeedRandom`, `PacketHandler_InputDelay`)
- `interBattle`: between battle rounds, after pre-battle negotiation and after end-of-battle synchronization (`DoConfirmSettings`, `tactrans.c`, `negotiateReadyConnections`)
- `selectShip`: ship-selection screen during battle handoff or between losses (`pickmele.c:780-793`, `PacketHandler_SelectShip`)
- `inBattle`: active battle input exchange (`PacketHandler_BattleInput`)
- `endingBattle`: first stage of end-of-battle synchronization where frame counts are exchanged (`tactrans.c:117-149`)
- `endingBattle2`: second stage of end-of-battle synchronization waiting for final ready rendezvous (`tactrans.c:108-114`, `224-225`)

### State-dependent protocol validity

Helper predicates in `netmisc.h` encode current protocol assumptions:

- `handshakeMeaningful(state)` is true only for `NetState_inSetup`
- `readyFlagsMeaningful(state)` is true for `init`, `preBattle`, `selectShip`, `interBattle`, `inBattle`, `endingBattle`, and `endingBattle2`
- `NetState_battleActive(state)` in `netstate.h:72-76` is true for `inBattle`, `endingBattle`, and `endingBattle2`

Packet handlers enforce these assumptions with `testNetState()` and reject packets received in invalid states as protocol errors (`packethandlers.c:41-52` and throughout the file).

## Current connection-local state flags

`netconnection.h:95-143` defines the connection-local control flags carried in `conn->stateFlags`:

- top-level booleans:
  - `connected`
  - `disconnected`
  - `discriminant`
- handshake flags:
  - `handshake.localOk`
  - `handshake.remoteOk`
  - `handshake.canceling`
- ready flags:
  - `ready.localReady`
  - `ready.remoteReady`
- reset flags:
  - `reset.localReset`
  - `reset.remoteReset`
- agreement flags:
  - currently only `agreement.randomSeed`
- negotiated per-connection values:
  - `inputDelay`
  - `checksumInterval` when checksum support is enabled

The `discriminant` flag is established asymmetrically on connection success and is documented in `netconnection.h:126-130` as a tie-breaker that remains fixed for the life of the connection.

## Protocol version negotiation and compatibility behavior

### `Init` packet contents

`Packet_Init` in `packet.h:104-116` contains:

- protocol version major/minor (`uint8` each)
- padding word
- UQM version major/minor/patch (`uint8` each)
- trailing padding byte

`Packet_Init_create()` in `packet.c:66-77` fills those fields from compile-time constants and `uqmversion.h`.

### Validation behavior on receipt

`PacketHandler_Init()` in `packethandlers.c:55-118` performs the current compatibility checks:

1. packet only valid in `NetState_init` while remote ready is not already set
2. exact protocol major/minor match is required
3. remote UQM version must be at least `NETPLAY_MIN_UQM_VERSION_*`

On either version failure path, the code:

- queues `AbortReason_versionMismatch` via `sendAbort()`
- emits UI feedback through `abortFeedback()`
- logs an error
- returns `-1` with `errno = ENOSYS`

A successful `Init` receipt does not itself advance state. It signals remote readiness via `Netplay_remoteReady()`, allowing the init ready rendezvous to complete.

## Current packet model and wire format

### Packet framing

All packets begin with `PacketHeader` in `packet.h:75-78`:

- `uint16 len`
- `uint16 type`

The length and type are serialized in network byte order. Helpers in `packet.h:85-99` use `ntoh16()` from `libs/network/bytesex.h`.

`Packet_create()` in `packet.c:45-60` sets both header fields and enforces that extra payload size is padded to a multiple of 4 bytes.

The packet comment in `packet.h:72-74` explicitly requires packet sizes to be multiples of 4 bytes and fields to stay aligned.

### Packet type list

`packet.h:24-43` defines the current packet types:

1. `PACKET_INIT`
2. `PACKET_PING`
3. `PACKET_ACK`
4. `PACKET_READY`
5. `PACKET_FLEET`
6. `PACKET_TEAMNAME`
7. `PACKET_HANDSHAKE0`
8. `PACKET_HANDSHAKE1`
9. `PACKET_HANDSHAKECANCEL`
10. `PACKET_HANDSHAKECANCELACK`
11. `PACKET_SEEDRANDOM`
12. `PACKET_INPUTDELAY`
13. `PACKET_SELECTSHIP`
14. `PACKET_BATTLEINPUT`
15. `PACKET_FRAMECOUNT`
16. `PACKET_CHECKSUM`
17. `PACKET_ABORT`
18. `PACKET_RESET`

`packet.c:31-52` maps each type to its minimum length, handler, and debug name.

### Per-packet payloads

Current packet payloads from `packet.h`:

- `Init`: protocol version + UQM version
- `Ping`: `uint32 id`
- `Ack`: `uint32 id`
- `Ready`: no payload
- `SeedRandom`: `uint32 seed`
- `InputDelay`: `uint32 delay`
- `Fleet`: `uint8 side`, `uint16 numShips`, followed by `FleetEntry[]`
- `TeamName`: `uint8 side`, followed by NUL-terminated `name[]` plus padding
- `Handshake0`: no payload
- `Handshake1`: no payload
- `HandshakeCancel`: no payload
- `HandshakeCancelAck`: no payload
- `SelectShip`: `uint16 ship` where `~0` is reserved for random selection
- `BattleInput`: `uint8 state`
- `FrameCount`: `uint32 frameCount`
- `Checksum`: `uint32 frameNr`, `uint32 checksum`
- `Abort`: `uint16 reason`
- `Reset`: `uint16 reason`

### Fleet and team-name payload conventions

`Packet_Fleet_create()` in `packet.c:102-119`:

- writes `side`
- writes `numShips` in network order
- stores an array of `(index, ship)` entries
- pads the tail with zero bytes

`Packet_TeamName_create()` in `packet.c:124-142`:

- writes `side`
- copies the name bytes without a caller-provided terminator
- appends a terminating `\0`
- pads remaining bytes with zeroes to 4-byte alignment

Current fleet synchronization therefore sends sparse slot updates using one-entry `Fleet` packets (`sendFleetShip`) or full snapshots using N-entry `Fleet` packets (`sendFleet`).

## Current receive path

### Buffering and packet extraction

`dataReadyCallback()` in `netrcv.c:108-168` is installed as the `NetDescriptor` read callback when a connection becomes established.

Behavior:

- receives into `conn->readBuf`/`conn->readEnd`
- closes the descriptor on EOF (`recv == 0`)
- handles `EWOULDBLOCK`/`EAGAIN` by returning without error
- retries on `EINTR`
- on fatal socket error, invokes `NetConnection_doErrorCallback()` and closes the descriptor
- passes accumulated data to `dataReceivedMulti()`
- memmoves unconsumed trailing bytes to the front of the read buffer

`dataReceivedSingle()` in `netrcv.c:31-83` parses one packet at a time:

- waits for a full header
- validates packet type
- validates header length against packet type minimum length
- waits for the full packet if incomplete
- dispatches to the packet handler from `packetTypeData[type].handler`

Malformed packet type or impossible length field yields `EBADMSG` and closes the connection upstream.

### Receive-side statistics/debugging

When `NETPLAY_STATISTICS` is enabled, `dataReceivedSingle()` increments:

- total packets received
- per-type packet counts

Debug logging intentionally suppresses battle input and checksum spam (`netrcv.c:57-74`).

## Current send path and packet queueing

### Queue-first sending model

Netplay mostly does not send packets immediately. `packetsenders.c` creates packet structs and calls `queuePacket()`.

`PacketQueue` in `packetq.h:30-38` is a singly linked FIFO with:

- `size`
- `first`
- `end`

`queuePacket()` in `packetq.c:52-87` appends the packet to the connection's queue and increments queue size.

### Flushing behavior

`flushPacketQueue()` in `packetq.c:117-131` walks the queue in order and calls `sendPacket()` for each packet. Sent packets are deleted; the first unsent packet remains in place if an error occurs.

`sendPacket()` in `netsend.c:31-87`:

- asserts the connection is connected
- loops until the full packet is written with `Socket_send()`
- retries on `EINTR`
- treats `ECONNRESET` as connection loss
- logs and returns `-1` on other send failures
- updates send statistics when enabled

There is no partial-send state kept across flushes; `sendPacket()` loops until the packet is fully written or fails.

### Global queue flush orchestration

`flushPacketQueues()` in `netmelee.c:118-140` iterates all connected connections, flushes each queue, and closes the connection on errors other than `EAGAIN`/`EWOULDBLOCK`.

`netInputAux()` in `netmelee.c:82-95` sequences transport/callback processing in this order:

1. `NetManager_process(&timeoutMs)`
2. `Async_process()`
3. `flushPacketQueues()`
4. `Callback_process()`

The comment in `netmelee.c` explains the final `Callback_process()` ordering: a flush may discover a disconnect and queue another callback, which must be processed before any subsequent flush touches the now-invalid socket.

## Current handshake / confirmation protocol

The confirmation protocol is implemented partly in `proto/npconfirm.c` and partly in handshake packet handlers in `packethandlers.c`.

### Purpose and state scope

`handshakeMeaningful()` in `netmisc.h:56-59` restricts confirmation to `NetState_inSetup`.

This protocol is used by the setup screen when the user presses Start Game. `StartMeleeButtonPressed()` in `melee.c:1521-1577` validates that network-controlled players are connected and in `NetState_inSetup`, then calls `confirmConnections()`.

### Local confirmation initiation

`Netplay_confirm()` in `proto/npconfirm.c:28-55`:

- requires `handshakeMeaningful(state)`
- rejects duplicate local confirmation with `EINVAL`
- sets `handshake.localOk = true`
- sends:
  - nothing yet if a prior cancel is still awaiting acknowledgement,
  - `Handshake1` if `remoteOk` is already true,
  - otherwise `Handshake0`

### Confirmation cancellation

`Netplay_cancelConfirmation()` in `proto/npconfirm.c:57-82`:

- requires handshake to be meaningful
- rejects cancellation if not locally confirmed
- clears `handshake.localOk`
- if a previous cancellation is already in flight, sends nothing
- otherwise sets `handshake.canceling = true` and sends `HandshakeCancel`

`DoConfirmSettings()` cancels confirmations when the user presses cancel or moves the cursor (`melee.c:1288-1312`).

### Incoming handshake packets

Handlers in `packethandlers.c` implement the protocol:

- `PacketHandler_Handshake0()` sets `remoteOk = true`; if local is already confirmed and not canceling, sends `Handshake1`
- `PacketHandler_Handshake1()` has two branches:
  - if canceling, it only records `remoteOk = true`
  - otherwise it clears both confirmation flags, may send a compensating `Handshake1` if no prior `Handshake0` was seen, and calls `handshakeComplete()`
- `PacketHandler_HandshakeCancel()` requires `remoteOk` to be set, then clears it and sends `HandshakeCancelAck`
- `PacketHandler_HandshakeCancelAck()` clears `canceling`; if local confirmation is still desired, it resumes by sending `Handshake1` or `Handshake0` depending on `remoteOk`

### Completion effect

`handshakeComplete()` in `packethandlers.c:271-279`:

- asserts both sides had confirmed and were not canceling
- requires current state `NetState_inSetup`
- sets state to `NetState_preBattle`

This state change is what `numPlayersReady()` in `melee.c:1242-1269` watches. A network-controlled player is considered done confirming once its connection state is greater than `NetState_inSetup`.

### UI-level invalidation on remote setup changes

When a remote `Fleet` or `TeamName` packet arrives while local confirmation is active, the handlers call:

- `Netplay_cancelConfirmation(conn)`
- `confirmationCancelled(meleeState, conn->player)`

This is implemented in `PacketHandler_Fleet()` (`packethandlers.c:160-165`) and `PacketHandler_TeamName()` (`packethandlers.c:209-214`). `confirmationCancelled()` in `melee.c:2113-2126` shows a reconfirmation message and, if currently inside `DoConfirmSettings`, kicks the menu back to ordinary setup mode.

## Current ready protocol

The generic ready protocol is implemented in `proto/ready.c`.

### Semantics

`Netplay_localReady()` (`ready.c:45-71`):

- requires a meaningful ready state and no existing local ready flag
- stores a callback and callback arg on the connection
- optionally sends a `Ready` packet
- if the remote side is not yet ready, sets `localReady = true` and returns false
- if the remote side is already ready, clears `remoteReady`, invokes the callback immediately, and returns true

`Netplay_remoteReady()` (`ready.c:74-92`):

- requires a meaningful ready state and no existing remote ready flag
- if local is not yet ready, sets `remoteReady = true` and returns false
- if local is ready, clears `localReady`, invokes the callback immediately, and returns true

The callback storage is one-shot. `Netplay_bothReady()` clears the stored callback before invoking it (`ready.c:25-43`).

### Where ready is used in current code

The current code uses ready as a generic synchronization primitive for several phases:

- init phase after `Init` packet exchange (`NetMelee_connectCallback`, `PacketHandler_Init`)
- pre-battle negotiation in `DoConfirmSettings()` through `negotiateReadyConnections(true, NetState_interBattle)` (`melee.c:1371-1380`)
- battle start in `battle.c:458-465` through `negotiateReadyConnections(true, NetState_inBattle)`
- ship-selection phase in `pickmele.c:780-793` through `negotiateReadyConnections(true, NetState_selectShip)`
- end-of-battle synchronization in `tactrans.c:157-167`, `193-225`

### Blocking wrappers around ready

`netmelee.c` provides blocking helpers that poll network input while waiting:

- `negotiateReady()` / `negotiateReadyConnections()` (`netmelee.c:489-581`)
- `waitReady()` (`netmelee.c:583-632`)

These wrappers drive `DoInput()` with tiny state objects whose `InputFunc` calls `netInputBlocking(NETWORK_POLL_DELAY)` and exits when a connection drops or the ready callback marks the operation done.

## Current reset protocol

The reset protocol is implemented in `proto/reset.c` and used to abort a running or pending game back to setup.

### Protocol semantics in the code comments

The detailed comment block at the top of `reset.c:24-60` describes the intended behavior:

- a reset packet indicates the game should return to the SuperMelee fleet setup menu
- reset is complete once a reset has both been sent and received
- if one side receives reset before it sent one, it confirms by sending reset back
- gameplay packets are no longer meaningful once reset has started
- incoming gameplay packets are ignored after remote reset is seen
- completion occurs when both `localReset` and `remoteReset` are set and a reset callback has been registered

### Local and remote reset entry points

`Netplay_localReset()` (`reset.c:104-117`):

- asserts local reset not already set
- sets `localReset = true`
- if remote reset is already set, just tries the completion condition
- otherwise sends `Reset` and calls `Netplay_connectionReset()` for feedback

`Netplay_remoteReset()` (`reset.c:119-133`):

- asserts remote reset was not already set
- sets `remoteReset = true`
- if local reset was not already set, sends a confirming `Reset`, sets `localReset = true`, and calls `Netplay_connectionReset()` with `byRemote = true`
- then checks completion condition

### Reset callbacks and waiting

`Netplay_setResetCallback()` (`reset.c:96-102`) stores the reset callback and immediately re-checks completion.

`waitReset()` / `waitResetConnections()` in `netmelee.c:636-726` install per-connection reset callbacks and block through `DoInput()` until the reset is complete or the connection drops. If local reset has not yet been sent, `waitReset()` initiates `ResetReason_manualReset` itself before blocking.

### UI and game-level consequences

`Netplay_connectionReset()` in `reset.c:63-84` only shows feedback for states from `preBattle` onward. For gameplay-related states it calls `resetFeedback()` in `melee.c`.

`resetFeedback()` in `melee.c:2212-2237`:

- flushes packet queues immediately so a locally queued reset confirmation is not left unsent
- suppresses UI for locally initiated manual reset
- otherwise shows a reason-specific message
- sets `GLOBAL(CurrentActivity) |= CHECK_ABORT`

Battle cleanup then uses `waitResetConnections(NetState_inSetup)` in `battle.c:475-489` so a reset returns the game to setup instead of escaping to the main menu.

## Current abort protocol

`Abort` is simpler than `Reset`.

### Reasons

`packet.h:47-55` defines current abort reasons:

- `AbortReason_unspecified`
- `AbortReason_versionMismatch`
- `AbortReason_invalidHash`
- `AbortReason_protocolError`

The current code actively uses version mismatch. `invalidHash` exists in the enum and in UI strings, but the code read for this subsystem did not show hash/signature validation logic using it.

### Behavior

`sendAbort()` queues an `Abort` packet (`packetsenders.c:184-190`).

`PacketHandler_Abort()` in `packethandlers.c:628-634`:

- calls `abortFeedback(conn, packet->reason)`
- returns `-1`, causing upstream connection close

The UI text comes from `abortReasonString()` / `abortFeedback()` in `melee.c:2157-2190`.

The subsystem also exposes `sendAbortConnections()` in `netmelee.c:440-456` to broadcast an abort reason to all connected peers.

## Current setup synchronization behavior

### Setup notifications

`notify.c` defines setup-screen notification primitives valid in `NetState_inSetup`:

- `Netplay_Notify_setTeamName()`
- `Netplay_Notify_setFleet()`
- `Netplay_Notify_setShip()`

All three assert that local confirmation is not currently active (`!conn->stateFlags.handshake.localOk`).

`notifyall.c` broadcasts those notifications to every connected connection currently in `NetState_inSetup`.

### Bootstrap full-team sync on connect/reentry

`Melee_bootstrapSyncTeam()` in `melee.c:2460-2486` sends:

1. full fleet snapshot via `Netplay_NotifyAll_setFleet()`
2. then current team name via `Netplay_NotifyAll_setTeamName()`
3. and updates the per-slot/per-name sent-state mirrors in `MeleeSetup`

This is called from `NetMelee_enterState_inSetup()` after initial connection establishment.

### Local setup edits

`Melee_LocalChange_ship()` and `Melee_LocalChange_teamName()` in `melee.c:2374-2423` implement the local side of a turn-based synchronization algorithm. If no outstanding sent value exists (`MELEE_UNSET` for ships, `NULL` for team names), they:

- apply the local change,
- notify peers,
- and record the sent value in `MeleeSetup`'s sent-state tracking

Whole-fleet/team replacement delegates to those per-field operations.

### Remote setup edits and conflict resolution

`Melee_RemoteChange_ship()` and `Melee_RemoteChange_teamName()` in `melee.c:2495-2635` implement a stateful conflict-resolution scheme using the sent-state mirrors.

Observed behavior from the code:

- If no local outstanding sent value exists, a remote value is applied locally and echoed back.
- If a local outstanding sent value exists, receipt ends the current “turn” by clearing or replacing the sent-state marker.
- If remote and sent values differ, the code breaks ties by comparing `NetConnection_getPlayerNr(conn)` with `side`:
  - when they differ, local value wins
  - otherwise remote value wins and is adopted locally
- If a local unsent follow-up change exists after confirmation, it is sent immediately as the next turn

This logic is descriptive of the current C behavior. It is subtle, asymmetric, and tied to the current two-player model.

## Current pre-battle negotiation behavior

After all sides confirm setup in `DoConfirmSettings()` (`melee.c:1343-1387`), the current code performs three steps before starting battle.

### 1. Broadcast preferred input delay

`Netplay_NotifyAll_inputDelay(netplayOptions.inputDelay)` broadcasts the local preference.

### 2. Synchronize RNG seed

For each network-controlled player, if its connection has `discriminant == true`, the local side sends a newly generated random seed via `Netplay_Notify_seedRandom(conn, SeedRandomNumbers())`.

Incoming seed is handled by `PacketHandler_SeedRandom()` in `packethandlers.c:423-436`, which:

- only accepts `SeedRandom` in `NetState_preBattle`
- only accepts it on the non-discriminant side (`!conn->stateFlags.discriminant`)
- calls `updateRandomSeed(meleeState, conn->player, seed)`
- sets `agreement.randomSeed = true`

`updateRandomSeed()` in `melee.c:2106-2111` simply reseeds the game RNG with `TFB_SeedRandom(seed)`.

### 3. Ready rendezvous into `interBattle`

`negotiateReadyConnections(true, NetState_interBattle)` waits until every connected peer has completed this pre-battle exchange.

### 4. Determine actual battle input delay

`setupInputDelay()` in `netmelee.c:399-423` scans all connected connections, finds the maximum remote-advertised `inputDelay`, and then raises that to at least the local configured delay if any network player exists. The chosen result is written to the global battle input delay via `setBattleInputDelay()`.

Current behavior therefore chooses the max of all connected peers’ advertised delays and the local desired delay.

## Current battle input buffering and delivery

### Buffer model

`netinput.c` defines a cyclic `BattleInputBuffer` per player and a single global `BattleInput_inputDelay`.

`initBattleInputBuffers()` (`netinput.c:56-89`) computes buffer capacity as `inputDelay * 2 + 2` and pre-fills each player's buffer with `inputDelay` zero-input frames. The comments explain this as worst-case storage for skew between both sides.

### Receiving battle input

`PacketHandler_BattleInput()` in `packethandlers.c:493-519`:

- accepts input packets in `inBattle`, `endingBattle`, or `endingBattle2`
- converts `packet->state` to `BATTLE_INPUT_STATE`
- pushes the result into `getBattleInputBuffer(conn->player)`
- fails if the buffer is full

### Sending battle input

`Netplay_Notify_battleInput()` in `notify.c:47-53` asserts battle-active states and queues a `BattleInput` packet.

`Netplay_NotifyAll_battleInput()` in `notifyall.c:130-145` broadcasts the same local input to all connected peers.

### Consuming network-controlled input

`networkBattleInput()` in `netmelee.c:261-366` is the battle-input callback used by the battle/input layer for network-controlled players.

Behavior:

- tries to pop one input from the front of that player's input buffer
- if none is available, calls `netInput()` once without blocking
- if the connection died, sets `CHECK_ABORT` and returns zero input
- if still nothing arrived, calls `netInputBlocking(MAX_BLOCK_TIME)` with `MAX_BLOCK_TIME = 500`
- repeats until input is available, disconnect occurs, or global abort is set

This is the current stall behavior for insufficient remote input. The code includes a disabled debug block for intentionally maximizing lag.

## Current checksum / CRC sync verification

### CRC scope

With `NETPLAY_CHECKSUM` enabled, `battle.c:268-304` computes a CRC every `NETPLAY_CHECKSUM_INTERVAL` frames, currently every frame.

`crc_processState()` in `checksum.c:187-197` includes:

- RNG seed state (`crc_processRNG()`)
- display queue / element state (`crc_processDispQueue()`)

`crc_processELEMENT()` excludes background objects (`checksum.c:104-127`).

The CRC helpers in `crc.c` operate on fixed-width integers in little-to-defined order via manual byte feeding.

### Buffering model

There are two checksum buffer classes:

- one local buffer `localChecksumBuffer` (`checksum.c:32`)
- one per connection inside `NetConnection` (`netconnection.h:173-175`)

`ChecksumBuffer_init()` in `checkbuf.c:41-81` sizes the cyclic checksum buffer to hold the worst-case number of outstanding delayed checksums given input delay and checksum interval.

### Sending and storing checksums

At battle frame `n`, `battle.c:268-283`:

- computes checksum
- broadcasts it with `Netplay_NotifyAll_checksum(n, checksum)`
- flushes packet queues
- stores it locally with `addLocalChecksum(n, checksum)`

Remote checksum receipt is handled by `PacketHandler_Checksum()` in `packethandlers.c:553-624`.

That handler currently:

- only accepts checksums in battle-active states
- ignores them completely if reset is active
- validates frame number is on the expected checksum interval
- rejects checksums too far in the future or too far in the past relative to `battleFrameCount` and input delay
- does **not** close the connection on those range mismatches; it logs a warning and discards the checksum
- on acceptable frames, stores the checksum with `addRemoteChecksum()`

### Delayed verification behavior

After input processing, `battle.c:287-304` verifies the checksum for `battleFrameCount - delay` once enough delayed frames have elapsed.

`verifyChecksums()` in `checksum.c:262-295`:

- requires a local checksum to exist for the verification frame
- requires every connected peer checksum buffer to have a checksum for that frame
- compares local and remote checksums for equality
- logs `"Network connections have gone out of sync."` on mismatch and returns false

On verification failure, `battle.c`:

- sets `CHECK_ABORT`
- calls `resetConnections(ResetReason_syncLoss)`

So current desync handling is reset-based rather than direct connection abort.

## Current ship-selection synchronization

During battle-side ship picking, `pickmele.c:780-793` first runs `negotiateReadyConnections(true, NetState_selectShip)` so all netplay participants enter the select-ship phase together.

Remote ship-selection packets are handled by `PacketHandler_SelectShip()` in `packethandlers.c:466-491`, which:

- only accepts them in `NetState_selectShip`
- forwards them to `updateMeleeSelection(gms, conn->player, ship)`
- treats invalid selection as protocol error

`updateMeleeSelection()` in `pickmele.c:907-928` verifies that the player is currently selecting and not already done, applies the selection with `setShipSelected(..., false)`, and marks `remoteSelected = TRUE`.

Local selection results are broadcast by `reportShipSelected()` in `pickmele.c:931-947`, which loops all connected peers and calls `Netplay_Notify_shipSelected(conn, index)`.

## Current end-of-battle synchronization

The battle-end synchronization logic lives in `tactrans.c:152-227`.

Current protocol as documented in its own comment block:

1. while in `NetState_inBattle`, use the Ready protocol to learn when each side is ready to stop battle
2. in `NetState_endingBattle`, each side sends the frame number when it wants to end battle and keeps simulating until that point
3. after both local and remote frame counts are known, each side simulates until the maximum target frame count
4. use Ready again to signal reaching that target
5. then end battle

Implementation details:

- `readyToEndCallback()` sets `NetState_endingBattle`, updates `endFrameCount`, sends `FrameCount(battleFrameCount + 1)`, flushes immediately, and arms another local ready callback for stage 2 (`tactrans.c:117-149`)
- `PacketHandler_FrameCount()` stores the maximum seen remote frame count in `battleStateData->endFrameCount` and then calls `Netplay_remoteReady(conn)` (`packethandlers.c:521-550`)
- `readyToEnd2Callback()` sets state to `NetState_endingBattle2` (`tactrans.c:109-114`)
- `readyForBattleEndPlayer()` keeps simulation running until `battleFrameCount >= endFrameCount`, waits for stage transitions, then uses `negotiateReady(conn, true, NetState_interBattle)` to complete

This is all still part of the active C netplay behavior and is relied on by battle return-to-setup flow.

## Current notification system

There are two layers of “notification” in the current subsystem.

### Network notifications: local state to wire packets

`notify.c` provides per-connection notification functions.

`notifyall.c` provides fan-out helpers that iterate active connections and call the per-connection variant. Current broadcast helpers are:

- `Netplay_NotifyAll_setTeamName()`
- `Netplay_NotifyAll_setFleet()`
- `Netplay_NotifyAll_setShip()`
- `Netplay_NotifyAll_inputDelay()`
- `Netplay_NotifyAll_checksum()`
- `Netplay_NotifyAll_battleInput()`

These functions are the main bridge from SuperMelee/battle events into queued netplay packets.

### UI/user notifications: protocol events back to SuperMelee

Netplay also invokes UI feedback hooks defined in `melee.c`:

- `connectedFeedback()` on successful entry into setup (`netmisc.c:123`)
- `closeFeedback()` on connection close (`netmisc.c:97`)
- `errorFeedback()` on connection error (`netmisc.c:104`)
- `abortFeedback()` on incoming abort or version mismatch handling (`packethandlers.c:99`, `111`, `630`)
- `resetFeedback()` on reset initiation/receipt during gameplay (`reset.c:85`)
- `confirmationCancelled()` when remote setup edits invalidate confirmation (`packethandlers.c:213`, `263`)
- `updateRandomSeed()` when a remote seed arrives (`packethandlers.c:430`)

The UI behavior itself is outside netplay ownership, but these callbacks are part of the current subsystem's integration contract.

## Current lower-level network and callback dependencies

### `libs/net.h` aggregation layer

`sc2/src/libs/net.h` re-exports the pieces netplay depends on:

- `network/network.h`
- `network/netmanager/netmanager.h`
- `network/connect/connect.h`
- `network/connect/listen.h`
- `network/connect/resolve.h`

Netplay includes `libs/net.h` in `netmelee.c`, `netrcv.h`, and `netsend.c`.

### `NetDescriptor` and `NetManager`

`NetDescriptor` in `libs/network/netmanager/ndesc.h` is the lower-level descriptor object netplay attaches to each socket.

Relevant behavior from `ndesc.c`:

- `NetDescriptor_new()` registers the socket with `NetManager`
- per-descriptor read/write/exception/close callbacks can be installed
- `NetDescriptor_close()` unregisters and closes the socket, then schedules the close callback through the generic callback queue
- `NetDescriptor_setReadCallback()` activates/deactivates the descriptor in the underlying `NetManager`

`NetManager_process()` in the BSD implementation (`netmanager_bsd.c`) uses `select()` on active descriptors and dispatches read/write/exception callbacks. The Windows implementation does the equivalent with `WSAWaitForMultipleEvents()`.

Netplay itself never calls `select()` directly; it relies on `NetManager_process()` through `netInputAux()`.

### Generic callback queue

`libs/callback/callback.c` provides a thread-safe FIFO callback queue.

Important semantics for netplay:

- callbacks are processed in queue order
- callbacks queued from inside a callback are deferred until the next `Callback_process()` call
- descriptor close callbacks are scheduled through this queue (`ndesc.c:72-99`)

This is why `netInputAux()` explicitly calls `Callback_process()` after `flushPacketQueues()`.

### Alarm/timer and async layer

`libs/callback/alarm.c` implements alarms using `SDL_GetTicks()` and a heap.

`libs/callback/async.c` defines `Async_process()` and `Async_timeBeforeNextMs()`:

- first processes queued callbacks,
- then fires due alarms one by one, processing any callbacks they enqueue after each alarm.

`netInputBlocking()` in `netmelee.c:102-112` asks `Async_timeBeforeNextMs()` for the next timer deadline and shortens the network poll timeout so async alarms are serviced promptly.

## Current integration points with SuperMelee menus and battle flow

### Setup menu integration

SuperMelee setup flow depends directly on current netplay state:

- `StartMeleeButtonPressed()` checks for invalid combinations such as both sides being network-controlled or network+computer combinations (`melee.c:1501-1519`)
- it validates each network-controlled player has a connected `NetConnection` in `NetState_inSetup` (`melee.c:1521-1571`)
- it shows a waiting message if not all players are confirmed yet and calls `confirmConnections()` (`melee.c:1573-1577`)

### Confirmation/start loop integration

`DoConfirmSettings()` in `melee.c:1279-1396` acts as the menu-side polling loop for the handshake and pre-battle negotiation:

- it polls/disconnect-cleans via `closeDisconnectedConnections()` and `netInput()`
- it sleeps briefly each frame
- it waits until `numPlayersReady()` reports all players past `NetState_inSetup`
- then performs input-delay broadcast, RNG seed sync, ready rendezvous into `interBattle`, input-delay selection, and battle start

### Battle startup integration

`battle.c:437-466` hooks netplay into battle startup:

- `initBattleInputBuffers()`
- `initChecksumBuffers()`
- reset `battleFrameCount`
- `ResetWinnerStarShip()`
- `setBattleStateConnections(&bs)`
- `negotiateReadyConnections(true, NetState_inBattle)`

### Ship-pick integration

`pickmele.c:780-793` runs `negotiateReadyConnections(true, NetState_selectShip)` before ship selection proceeds.

### Battle-end/reset integration

`tactrans.c` drives the end-of-battle synchronization protocol.

`battle.c:475-489` uses `waitResetConnections(NetState_inSetup)` when a battle abort/reset occurred so control returns to the SuperMelee setup screen.

### Global cleanup integration

`FreeMeleeInfo()` in `melee.c:1411-1435` calls `closeAllConnections()` and then resets input delay back to `0` using `setupInputDelay(0)` after all connections have been closed.

## Current error handling characteristics

### Hard protocol errors

Malformed or state-invalid packets generally cause:

- packet handler returns `-1`
- `dataReceivedSingle()` propagates failure
- `dataReadyCallback()` calls `NetConnection_doErrorCallback()` and closes the descriptor

Examples include:

- invalid packet type or impossible packet length (`netrcv.c:42-55`)
- packets received in the wrong `NetState` (`packethandlers.c:testNetState`)
- duplicate/invalid reset-state combinations
- absurd input delay values (`packethandlers.c:450-460` rejects values above `BATTLE_FRAME_RATE`)
- invalid fleet ship IDs or slot indices (`packethandlers.c:167-188`)
- invalid remote ship selection (`packethandlers.c:482-489`)

### Soft checksum validation failures

Checksum packet range anomalies are currently soft failures: the handler logs and discards the packet without closing the connection (`packethandlers.c:577-617`).

Actual checksum mismatch during delayed verification escalates to a coordinated reset (`battle.c:297-300`).

### Connection failures and disconnects

Connection-establishment failures are surfaced through `NetMelee_errorCallback()` and then the connection is closed.

Runtime disconnects close the `NetDescriptor`, trigger `closeFeedback()`, and eventually remove the connection from the global array.

## Current notable constraints and observations grounded in code

- The implementation is effectively two-player. `NETPLAY_NUM_PLAYERS` is hard-coded to `2` in `netoptions.h:28-31`, and `NetMelee_enterState_inSetup()` explicitly comments “This only works with 2 players atm.” (`netmisc.c:126-128`).
- Transport is stream/TCP based, not datagram-based. The code uses `connectHostByName(..., IPProto_tcp, ...)`, `listenPort(..., IPProto_tcp, ...)`, `Socket_recv()`, and `Socket_send()`.
- Packet handlers are expected not to modify state on error, as documented in `packet.h:57-61`, but that is a convention rather than an externally enforced transaction system.
- There is no resend/reliability layer in netplay itself; reliability is delegated to TCP.
- The ping/ack packet types exist and are parsed/queued, but the current SuperMelee flow shown in the read code does not appear to use them for matchmaking, latency measurement, or liveness policy.
- `AbortReason_invalidHash` exists but no hash/signature validation path was identified in the read netplay sources.
- The connection-level `agreement.randomSeed` flag is set on both send and receive paths, but the higher-level code shown does not use that flag for an explicit “all agreements satisfied” gate.
- The state machine and ready/reset helpers rely heavily on nested callbacks and `DoInput()`-driven blocking wait loops rather than a centralized explicit scheduler.

## Key active files, functions, and structures

### Files

- `sc2/src/uqm/supermelee/netplay/netconnection.c`, `netconnection.h`, `nc_connect.ci`
- `sc2/src/uqm/supermelee/netplay/netmelee.c`, `netmelee.h`
- `sc2/src/uqm/supermelee/netplay/netstate.c`, `netstate.h`
- `sc2/src/uqm/supermelee/netplay/netrcv.c`, `netsend.c`, `packetq.c`
- `sc2/src/uqm/supermelee/netplay/packet.c`, `packet.h`, `packethandlers.c`, `packetsenders.c`
- `sc2/src/uqm/supermelee/netplay/proto/npconfirm.c`, `proto/ready.c`, `proto/reset.c`
- `sc2/src/uqm/supermelee/netplay/netinput.c`, `checksum.c`, `checkbuf.c`, `crc.c`
- `sc2/src/uqm/supermelee/netplay/notify.c`, `notifyall.c`

### Functions

- `NetConnection_open()`, `NetConnection_close()`, `NetConnection_setState()`
- `NetMelee_connectCallback()`, `NetMelee_closeCallback()`, `NetMelee_errorCallback()`
- `netInput()`, `netInputBlocking()`, `flushPacketQueues()`
- `negotiateReady()`, `negotiateReadyConnections()`, `waitReady()`
- `waitReset()`, `waitResetConnections()`, `resetConnections()`
- `PacketHandler_Init()` through `PacketHandler_Reset()`
- `Netplay_confirm()`, `Netplay_cancelConfirmation()`
- `Netplay_localReady()`, `Netplay_remoteReady()`
- `Netplay_localReset()`, `Netplay_remoteReset()`
- `networkBattleInput()`
- `setupInputDelay()`
- `verifyChecksums()`

### Data structures

- `NetConnection` (`netconnection.h:145-191`)
- `NetState` (`netstate.h:25-42`)
- `NetStateFlags`, `HandShakeFlags`, `ReadyFlags`, `ResetFlags`, `Agreement` (`netconnection.h:95-143`)
- `BattleStateData` (`netmisc.h:32-39`)
- `PacketHeader`, all `Packet_*` wire structs (`packet.h`)
- `PacketQueue` (`packetq.h:30-38`)
- `BattleInputBuffer` (`netinput.h:28-36`)
- `ChecksumBuffer` / `ChecksumEntry` (`checkbuf.h:42-63`)

## Concise conclusion

The current C netplay subsystem is a direct TCP peer-to-peer protocol layered on top of UQM's `NetDescriptor`/`NetManager` and callback/alarm infrastructure. It owns a per-connection state machine, wire packet schema, setup synchronization, confirmation/ready/reset protocols, battle input buffering, and checksum-based sync checks. SuperMelee still owns menu flow, team-state mutation, UI feedback, and battle orchestration around it, while battle owns simulation and calls into netplay for input, checksums, and end-of-battle synchronization. The implementation is clearly active, two-player-oriented, callback-heavy, and grounded in the packet/state behaviors described above.