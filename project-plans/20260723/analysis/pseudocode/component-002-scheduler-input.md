# Pseudocode 002: Scheduler, Input, and Menu Observer

Normative table/timelines: `../authoritative-execution-contract.md` §§2-3,5.

```text
101: DIRECT_CALLBACK_ENTRY(kind)
102:   saturating_increment ABI_ENTRY[kind]
103:   IF acquire ACTIVE == false RETURN neutral_without_TLS_allocation_lock_or_work
104:   saturating_increment ACTIVE_GATE_ENTRY[kind]
105:   IF callback_depth != 0 FALLBACK(Reentrant); RETURN conservative(kind)
106:   SET depth guard; CATCH_UNWIND around lines 107-119; restore depth always
107:   LOCK runtime; poison => terminal-only plan
108:   PURE_REDUCE: checked-increment applicable callback ordinal, sample time once
109:   APPLY priority overflow -> input >= max -> present >= max -> wall >= max -> clock
110:   IF admitted RESERVE checked sequence/state_version/generation and EffectPlan
111:   UNLOCK unconditionally
112:   EXECUTE C/key/getter effects; release-update owned-key mirror per success
113:   ORDERED_PUBLISH_OR_CANCEL reservation outside runtime lock
114:   LOCK runtime; COMMIT only matching sequence/version/generation/terminal mirror
115:   UNLOCK before any next effect; repeat only allowed zero-callback actions
116:   RETURN typed stop/continue
117: ON error/panic: terminal CAS; clear capture; cancel reservation
118:   release mirrored keys + OR CHECK_ABORT outside locks
119:   RETURN conservative(kind)
120:
121: DO_INPUT_ITERATION
122:   existing pumps; TaskSwitch
123:   service_stop = SERVICE_INPUT
124:   UpdateInputState exactly once
125:   observation_stop = AFTER_INPUT_UPDATE using same shell
126:   IF service_stop OR observation_stop BREAK before journal/sound/callback/InputFunc
127:
128: MAIN_MENU_NAVIGATE
129:   draw new selection
130:   state.cur_state = new_item.as_u8
131:   ops.sync_cur_state(state.cur_state)
132:   control = panic-contained typed observer(from,new_item)
133:   IF control == Stop RETURN Stop before timestamp/sleep/later frame work
134:   update last_input_time; RETURN Continue
135: DO_RESTART_FRAME propagates Stop immediately
136: RUST_DO_RESTART_FRAME catches complete shell and maps Stop/panic to 0
137: RUST_START_GAME catches complete orchestration shell and maps terminal/panic to 0
138:
139: DIRECT_UPDATE_CONSUMERS
140:   inventory ConfirmExit, BackgroundInitKernel, MeleeGameOver, AnyButtonPress
141:   include current talk_segue.rs do_talk_segue -> c_UpdateInputState
142:   safe_point before update; post_update after; terminal skips all ordinary action
143:   use real site or shared production helper called by every site and harness
```
