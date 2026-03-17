# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-NETPLAY.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed and passed
- Expected artifacts: complete analysis documents

## Purpose
Produce algorithmic pseudocode for every significant component in the netplay subsystem. Each pseudocode block must be numbered and include validation, error handling, ordering constraints, and integration boundaries.

## Pseudocode Components

### Component 001: NetState Machine (state.rs)
```
01: ENUM NetState { Unconnected, Connecting, Init, InSetup, PreBattle,
02:                 InterBattle, SelectShip, InBattle, EndingBattle, EndingBattle2 }
03:
04: FUNCTION is_handshake_meaningful(state) -> bool
05:   RETURN state == InSetup
06:
07: FUNCTION is_ready_meaningful(state) -> bool
08:   RETURN state IN {Init, PreBattle, SelectShip, InterBattle,
09:                    InBattle, EndingBattle, EndingBattle2}
10:
11: FUNCTION is_battle_active(state) -> bool
12:   RETURN state IN {InBattle, EndingBattle, EndingBattle2}
13:
14: FUNCTION validate_transition(from, to) -> Result
15:   MATCH (from, to):
16:     (Unconnected, Connecting) -> Ok
17:     (Connecting, Init) -> Ok
18:     (Connecting, Unconnected) -> Ok
19:     (Init, InSetup) -> Ok
20:     (Init, Unconnected) -> Ok
21:     (InSetup, PreBattle) -> Ok
22:     (PreBattle, InterBattle) -> Ok
23:     (InterBattle, SelectShip) -> Ok
24:     (InterBattle, InBattle) -> Ok
25:     (SelectShip, InBattle) -> Ok
26:     (InBattle, EndingBattle) -> Ok
27:     (EndingBattle, EndingBattle2) -> Ok
28:     (EndingBattle2, InterBattle) -> Ok
29:     (_, InSetup) -> Ok
30:     (_, Unconnected) -> Ok
31:     _ -> Err(InvalidTransition)
```

### Component 002: Packet Codec (packet/codec.rs)
```
32: CONST HEADER_SIZE = 4
33:
34: FUNCTION serialize_packet(packet) -> Vec<u8>
35:   LET payload = serialize_payload(packet)
36:   LET padded_len = round_up_to_4(HEADER_SIZE + payload.len())
37:   LET mut buf = Vec::with_capacity(padded_len)
38:   WRITE u16 big-endian: padded_len as u16
39:   WRITE u16 big-endian: packet.type_id()
40:   WRITE payload bytes
41:   PAD with zeroes to padded_len
42:   RETURN buf
43:
44: FUNCTION deserialize_header(buf: &[u8]) -> Result<(u16, PacketType)>
45:   REQUIRE buf.len() >= HEADER_SIZE
46:   LET len = read_u16_be(buf[0..2])
47:   LET type_id = read_u16_be(buf[2..4])
48:   VALIDATE type_id is known PacketType
49:   VALIDATE len >= minimum_length(type_id)
50:   VALIDATE len is multiple of 4
51:   RETURN Ok((len, type_id))
52:
53: FUNCTION deserialize_packet(type_id, buf) -> Result<Packet>
54:   MATCH type_id:
55:     PACKET_INIT -> parse_init(buf)
56:     PACKET_FLEET -> parse_fleet(buf)
57:     PACKET_TEAMNAME -> parse_team_name(buf)
58:     PACKET_BATTLEINPUT -> parse_battle_input(buf)
59:     ... (all 18 types)
60:   VALIDATE all fields within bounds and canonical Rust aliases
61:   RETURN parsed packet
```

### Component 003: NetConnection (connection/net_connection.rs)
```
62: STRUCT NetConnection {
63:   player: PlayerId,
64:   state: NetState,
65:   state_flags: StateFlags,
66:   transport: Option<TcpStream>,
67:   read_buf: Vec<u8>,
68:   read_end: usize,
69:   packet_queue: PacketQueue,
70:   options: PeerOptions,
71:   ready_callback: Option<Box<dyn FnOnce()>>,
72:   reset_callback: Option<Box<dyn FnOnce()>>,
73:   battle_state: Option<BattleStateData>,
74:   stats: Option<NetStats>,
75: }
76:
77: FUNCTION new(player, options) -> NetConnection
78:   RETURN NetConnection { ... }
79:
80: FUNCTION set_state(&mut self, new_state) -> Result
81:   VALIDATE validate_transition(self.state, new_state)
82:   self.state = new_state
83:   RETURN Ok(())
84:
85: FUNCTION is_connected(&self) -> bool
86:   RETURN self.state_flags.connected && !self.state_flags.disconnected
```

### Component 004: Transport (connection/transport.rs)
```
87: FUNCTION connect_as_server(port, backlog) -> Result<TcpListener>
88:   LET listener = TcpListener::bind(("0.0.0.0", port))
89:   listener.set_nonblocking(true)
90:   RETURN listener
91:
92: FUNCTION accept_connection(listener) -> Result<TcpStream>
93:   LET (stream, addr) = listener.accept()
94:   stream.set_nonblocking(true)
95:   stream.set_nodelay(true)
96:   RETURN stream
97:
98: FUNCTION connect_as_client(host, port, timeout_ms) -> Result<TcpStream>
99:   LET addr = resolve(host, port)
100:  LET stream = TcpStream::connect_timeout(addr, Duration::from_millis(timeout_ms))
101:  stream.set_nonblocking(true)
102:  stream.set_nodelay(true)
103:  RETURN stream
104:
105: FUNCTION assign_discriminant(is_server) -> bool
106:  RETURN is_server
```

### Component 005: Packet Queue (packet/queue.rs)
```
107: STRUCT PacketQueue { packets: VecDeque<Vec<u8>> }
108:
109: FUNCTION enqueue(&mut self, serialized_packet: Vec<u8>)
110:   self.packets.push_back(serialized_packet)
111:
112: FUNCTION flush(&mut self, stream: &mut TcpStream) -> Result<usize>
113:   LET mut sent = 0
114:   WHILE let Some(packet) = self.packets.front():
115:     MATCH send_all(stream, packet):
116:       Ok(_) -> { self.packets.pop_front(); sent += 1 }
117:       Err(WouldBlock) -> BREAK
118:       Err(e) -> RETURN Err(e)
119:   RETURN Ok(sent)
```

### Component 006: Receive Path (packet/receive.rs)
```
120: FUNCTION receive_packets(conn: &mut NetConnection) -> Result<Vec<Packet>>
121:   LET mut packets = Vec::new()
122:   MATCH conn.transport.read(&mut conn.read_buf[conn.read_end..]):
123:     Ok(0) -> RETURN Err(ConnectionClosed)
124:     Ok(n) -> conn.read_end += n
125:     Err(WouldBlock) -> {}
126:     Err(Interrupted) -> {}
127:     Err(e) -> RETURN Err(e)
128:
129:   LET mut offset = 0
130:   WHILE offset + HEADER_SIZE <= conn.read_end:
131:     LET (pkt_len, pkt_type) = deserialize_header(&conn.read_buf[offset..])?
132:     IF offset + pkt_len as usize > conn.read_end:
133:       BREAK
134:     LET packet = deserialize_packet(pkt_type, &conn.read_buf[offset..offset+pkt_len])?
135:     packets.push(packet)
136:     offset += pkt_len as usize
137:
138:   IF offset > 0:
139:     conn.read_buf.copy_within(offset..conn.read_end, 0)
140:     conn.read_end -= offset
141:
142:   RETURN Ok(packets)
```

### Component 007: Ready Protocol (proto/ready.rs)
```
143: FUNCTION local_ready(conn, callback, send_packet) -> Result<bool>
144:   REQUIRE is_ready_meaningful(conn.state)
145:   REQUIRE !conn.state_flags.ready.local_ready
146:   IF send_packet:
147:     conn.packet_queue.enqueue(serialize(ReadyPacket))
148:   IF conn.state_flags.ready.remote_ready:
149:     conn.state_flags.ready.remote_ready = false
150:     both_ready(conn, callback)
151:     RETURN Ok(true)
152:   ELSE:
153:     conn.state_flags.ready.local_ready = true
154:     conn.ready_callback = Some(callback)
155:     RETURN Ok(false)
156:
157: FUNCTION remote_ready(conn) -> Result<bool>
158:   REQUIRE is_ready_meaningful(conn.state)
159:   REQUIRE !conn.state_flags.ready.remote_ready
160:   IF conn.state_flags.ready.local_ready:
161:     conn.state_flags.ready.local_ready = false
162:     LET cb = conn.ready_callback.take()
163:     both_ready(conn, cb)
164:     RETURN Ok(true)
165:   ELSE:
166:     conn.state_flags.ready.remote_ready = true
167:     RETURN Ok(false)
```

### Component 008: Confirmation Protocol (proto/confirm.rs)
```
168: FUNCTION confirm(conn) -> Result
169:   REQUIRE is_handshake_meaningful(conn.state)
170:   REQUIRE !conn.state_flags.handshake.local_ok
171:   conn.state_flags.handshake.local_ok = true
172:   IF conn.state_flags.handshake.canceling:
173:     RETURN Ok(())
174:   IF conn.state_flags.handshake.remote_ok:
175:     SEND Handshake1
176:   ELSE:
177:     SEND Handshake0
178:   RETURN Ok(())
179:
180: FUNCTION cancel_confirmation(conn) -> Result
181:   REQUIRE is_handshake_meaningful(conn.state)
182:   REQUIRE conn.state_flags.handshake.local_ok
183:   conn.state_flags.handshake.local_ok = false
184:   IF conn.state_flags.handshake.canceling:
185:     RETURN Ok(())
186:   conn.state_flags.handshake.canceling = true
187:   SEND HandshakeCancel
188:   RETURN Ok(())
```

### Component 009: Reset Protocol (proto/reset.rs)
```
189: FUNCTION local_reset(conn, reason) -> Result
190:   REQUIRE !conn.state_flags.reset.local_reset
191:   conn.state_flags.reset.local_reset = true
192:   IF conn.state_flags.reset.remote_reset:
193:     try_reset_complete(conn)
194:   ELSE:
195:     SEND Reset(reason)
196:     emit_reset_feedback(conn, reason, by_remote=false)
197:   RETURN Ok(())
198:
199: FUNCTION remote_reset(conn, reason) -> Result
200:   REQUIRE !conn.state_flags.reset.remote_reset
201:   conn.state_flags.reset.remote_reset = true
202:   IF !conn.state_flags.reset.local_reset:
203:     conn.state_flags.reset.local_reset = true
204:     SEND Reset(reason)
205:     emit_reset_feedback(conn, reason, by_remote=true)
206:   try_reset_complete(conn)
207:   RETURN Ok(())
```

### Component 010: Battle Input Buffer (input/buffer.rs)
```
208: STRUCT BattleInputBuffer {
209:   buf: Vec<u8>, head: usize, tail: usize, size: usize, capacity: usize,
210: }
211:
212: FUNCTION new(input_delay) -> BattleInputBuffer
213:   LET capacity = input_delay * 2 + 2
214:   LET mut buffer = BattleInputBuffer { ... }
215:   FOR _ in 0..input_delay:
216:     buffer.push(0)
217:   RETURN buffer
218:
219: FUNCTION push(&mut self, input: u8) -> Result
220:   REQUIRE self.size < self.capacity
221:   ...
222:
223: FUNCTION pop(&mut self) -> Option<u8>
224:   IF self.size == 0: RETURN None
225:   ...
```

### Component 011: Checksum Buffer & Verification (checksum/)
```
226: STRUCT ChecksumEntry { frame: u32, checksum: u32 }
227:
228: STRUCT ChecksumBuffer {
229:   entries: Vec<Option<ChecksumEntry>>, capacity: usize, checksum_interval: u32,
230: }
231:
232: FUNCTION new(input_delay, checksum_interval) -> ChecksumBuffer
233:   LET capacity = (input_delay * 2 / checksum_interval) + 2
234:   RETURN ChecksumBuffer { ... }
235:
236: FUNCTION add(&mut self, frame: u32, checksum: u32) -> Result
237:   LET idx = (frame / checksum_interval) % self.capacity
238:   self.entries[idx] = Some(ChecksumEntry { frame, checksum })
239:   RETURN Ok(())
240:
241: FUNCTION verify_checksums(frame, local_buf, remote_bufs) -> Result<bool>
242:   LET local = local_buf.get(frame)
243:   REQUIRE local.is_some()
244:   FOR remote_buf in remote_bufs:
245:     LET remote = remote_buf.get(frame)
246:     IF remote.is_none(): RETURN Ok(true)
247:     IF remote != local: RETURN Ok(false)
248:   RETURN Ok(true)
```

### Component 012: Connection Registry & Polling (connection/registry.rs)
```
249: STRUCT ConnectionRegistry {
250:   connections: [Option<NetConnection>; NUM_PLAYERS],
251:   num_connections: usize,
252: }
253:
254: FUNCTION poll_all(&mut self, timeout_ms: u32) -> Result<Vec<(PlayerId, Packet)>>
255:   LET mut packets = Vec::new()
256:   FOR player in 0..NUM_PLAYERS:
257:     IF let Some(conn) = &mut self.connections[player]:
258:       IF !conn.is_connected(): CONTINUE
259:       LET received = receive_packets(conn)?
260:       FOR packet in received:
261:         packets.push((player, packet))
262:   RETURN Ok(packets)
263:
264: FUNCTION flush_all(&mut self) -> Result
265:   FOR player in 0..NUM_PLAYERS:
266:     IF let Some(conn) = &mut self.connections[player]:
267:       IF !conn.is_connected(): CONTINUE
268:       conn.packet_queue.flush(&mut conn.transport)?
269:   RETURN Ok(())
```

### Component 013: Setup Conflict Resolution (integration/melee_hooks.rs)
```
270: FUNCTION resolve_setup_conflict(local_pending, remote_update, discriminant) -> Resolution
271:   REQUIRE remote_update.state == InSetup-equivalent
272:   IF local_pending is None:
273:     RETURN ApplyRemote
274:   IF local_pending.field != remote_update.field:
275:     RETURN ApplyRemoteAndKeepLocalPendingForOtherField
276:   IF local_pending.value == remote_update.value:
277:     RETURN NoOp
278:   // Crossing edit on same synchronized field
279:   IF discriminant == true:
280:     RETURN KeepLocalRejectRemote
281:   ELSE:
282:     RETURN ApplyRemoteInvalidateLocalConfirmation
283:
284: FUNCTION apply_setup_resolution(resolution)
285:   MATCH resolution:
286:     ApplyRemote -> emit Remote*Update
287:     ApplyRemoteInvalidateLocalConfirmation -> clear confirmation, emit ConfirmationInvalidated, emit Remote*Update
288:     KeepLocalRejectRemote -> do not mutate local canonical setup state
```

### Component 014: Shared Progress Loop (integration/*)
```
289: FUNCTION drive_progress_until(predicate, timeout_ms) -> Result
290:   LET deadline = now + timeout_ms
291:   LOOP:
292:     flush_all_queued_packets()
293:     LET received = registry.poll_all(SHORT_POLL_MS)?
294:     FOR (player, packet) in received:
295:       LET event = dispatch_packet(player, packet)?
296:       deliver_event_to_sink(event)
297:     run_deferred_ready_or_reset_callbacks_once()
298:     IF predicate(): RETURN Ok
299:     IF disconnect_or_abort_observed(): RETURN Err(...)
300:     IF now >= deadline: RETURN Err(Timeout)
301:     yield_or_sleep_shortly_if_no_progress()
```

### Component 015: Ship-Selection Semantic Validation Boundary (integration/melee_hooks.rs)
```
302: FUNCTION accept_remote_ship_selection(player, ship: ShipId) -> Result
303:   REQUIRE transport_validation_already_passed
304:   LET semantic_ok = supermelee_validate_remote_ship(player, ship)
305:   IF semantic_ok:
306:     commit_selection_to_owner(player, ship)
307:     RETURN Ok
308:   block_battle_handoff()
309:   initiate_reset(SyncLoss or protocol-defined invalid-selection reason)
310:   RETURN Err(InvalidRemoteSelection)
```

## Pseudocode Files to Create

Create these files in `project-plans/20260311/netplay/analysis/pseudocode/`:
- `component-001-state-machine.md`
- `component-002-packet-codec.md`
- `component-003-connection.md`
- `component-004-transport.md`
- `component-005-packet-queue.md`
- `component-006-receive-path.md`
- `component-007-ready-protocol.md`
- `component-008-confirm-protocol.md`
- `component-009-reset-protocol.md`
- `component-010-battle-input.md`
- `component-011-checksum.md`
- `component-012-registry.md`
- `component-013-setup-conflict-resolution.md`
- `component-014-progress-loop.md`
- `component-015-ship-selection-boundary.md`

## Verification Commands

```bash
# No code changes in pseudocode phase
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Success Criteria
- [ ] All 15 pseudocode components written with numbered lines
- [ ] Validation points marked at every entry
- [ ] Error handling paths explicit
- [ ] Integration boundaries annotated
- [ ] Side effects documented
- [ ] Progress-loop and setup-conflict algorithms are explicit enough to implement without reinterpretation

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P02.md`
