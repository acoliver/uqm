# process.c process-loop pseudocode (Phase P02)

## alloc_element()
1. `fn alloc_element() -> Helement {`
2. `    let h_element = alloc_link(&mut disp_q); // FFI:`
3. `    if h_element == 0 {`
4. `        // [VALIDATE] allocation failure from link pool`
5. `        return 0;`
6. `    }`
7. `    let mut element_ptr: *mut Element = null_mut(); // Phase1: Element`
8. `    lock_element(h_element, &mut element_ptr); // FFI:`
9. `    // [VALIDATE] locked element pointer must be non-null`
10. `    if element_ptr.is_null() {`
11. `        // Error path: defensive fallback (C assumes valid lock)`
12. `        unlock_element(h_element); // FFI:`
13. `        free_link(&mut disp_q, h_element); // FFI:`
14. `        return 0;`
15. `    }`
16. `    memset(element_ptr, 0, size_of::<Element>()); // FFI: C memset`
17. `    (*element_ptr).prim_index = alloc_display_prim(); // FFI:`
18. `    if (*element_ptr).prim_index == END_OF_LIST {`
19. `        log_add(log_error, "AllocElement: Out of display prims!"); // FFI:`
20. `        explode(); // FFI: fatal abort path`
21. `        // [VALIDATE] no normal recovery in C after explode`
22. `    }`
23. `    set_prim_type(&mut display_array[(*element_ptr).prim_index], no_prim); // FFI: // Phase1: display_array/Prim`
24. `    unlock_element(h_element); // FFI:`
25. `    return h_element;`
26. `}`

## free_element()
27. `fn free_element(h_element: Helement) {`
28. `    if h_element == 0 {`
29. `        // [VALIDATE] null handle is a no-op`
30. `        return;`
31. `    }`
32. `    let mut element_ptr: *mut Element = null_mut(); // Phase1: Element`
33. `    lock_element(h_element, &mut element_ptr); // FFI:`
34. `    if !element_ptr.is_null() {`
35. `        free_display_prim((*element_ptr).prim_index); // FFI:`
36. `    } else {`
37. `        // Error path: defensive; C assumes lock provides valid pointer`
38. `    }`
39. `    unlock_element(h_element); // FFI:`
40. `    free_link(&mut disp_q, h_element); // FFI:`
41. `}`

## setup_element()
42. `fn setup_element(element_ptr: &mut Element) { // Phase1: Element`
43. `    element_ptr.next = element_ptr.current;`
44. `    if colliding_element(element_ptr) { // FFI: macro/helper`
45. `        init_intersect_start_point(element_ptr); // FFI:`
46. `        init_intersect_end_point(element_ptr); // FFI:`
47. `        init_intersect_frame(element_ptr); // FFI:`
48. `    }`
49. `}`

## untarget()
50. `fn untarget(removee: &mut Element) { // Phase1: Element`
51. `    // [VALIDATE] only meaningful when removee has parent references`
52. `    let mut h_scan = get_head_element(); // FFI:`
53. `    while h_scan != 0 {`
54. `        let mut scan_ptr: *mut Element = null_mut(); // Phase1: Element`
55. `        lock_element(h_scan, &mut scan_ptr); // FFI:`
56. `        let h_next = get_succ_element(scan_ptr); // FFI:`
57. `        if scan_ptr == (&mut *removee as *mut Element) {`
58. `            unlock_element(h_scan); // FFI:`
59. `            h_scan = h_next;`
60. `            continue;`
61. `        }`
62. `        // clear target/tracking refs that point at removee`
63. `        // [VALIDATE] any field-equivalent to pParent / hTarget / lock-on refs must be nulled`
64. `        clear_tracking_refs_if_pointing_to_removee(scan_ptr, removee); // Phase1: relationship semantics`
65. `        unlock_element(h_scan); // FFI:`
66. `        h_scan = h_next;`
67. `    }`
68. `}`

## remove_element()
69. `fn remove_element(h_element: Helement) {`
70. `    if h_element == 0 {`
71. `        // [VALIDATE] null handle no-op`
72. `        return;`
73. `    }`
74. `    let mut element_ptr: *mut Element = null_mut(); // Phase1: Element`
75. `    lock_element(h_element, &mut element_ptr); // FFI:`
76. `    if element_ptr.is_null() {`
77. `        // Error path: defensive unlock/remove fallback`
78. `        unlock_element(h_element); // FFI:`
79. `        remove_link(&mut disp_q, h_element); // FFI:`
80. `        return;`
81. `    }`
82. `    untarget(&mut *element_ptr);`
83. `    unlock_element(h_element); // FFI:`
84. `    remove_link(&mut disp_q, h_element); // FFI:`
85. `}`

## pre_process()
86. `fn pre_process(element_ptr: &mut Element) { // Phase1: Element`
87. `    let mut state_flags: ElementFlags; // Phase1: ElementFlags`
88. `    if element_ptr.life_span == 0 {`
89. `        if !element_ptr.p_parent.is_null() {`
90. `            untarget(element_ptr);`
91. `        }`
92. `        element_ptr.state_flags |= disappearing;`
93. `        if let Some(death_func) = element_ptr.death_func {`
94. `            death_func(element_ptr); // FFI: function pointer callback`
95. `        }`
96. `    }`
97. `    state_flags = element_ptr.state_flags;`
98. `    if (state_flags & disappearing) == 0 {`
99. `        if (state_flags & appearing) != 0 {`
100. `            setup_element(element_ptr);`
101. `            if (state_flags & player_ship) != 0 {`
102. `                state_flags &= !appearing; // ship is preprocessed immediately`
103. `            }`
104. `        }`
105. `        if element_ptr.preprocess_func.is_some() && (state_flags & appearing) == 0 {`
106. `            element_ptr.preprocess_func.unwrap()(element_ptr); // FFI: callback`
107. `            state_flags = element_ptr.state_flags;`
108. `            if (state_flags & changing) != 0 && colliding_element(element_ptr) {`
109. `                init_intersect_frame(element_ptr); // FFI:`
110. `            }`
111. `        }`
112. `        if (state_flags & ignore_velocity) == 0 {`
113. `            let (mut delta_x, mut delta_y): (Size, Size) = (0, 0); // Phase1: SIZE`
114. `            get_next_velocity_components(&element_ptr.velocity, &mut delta_x, &mut delta_y, 1); // FFI:`
115. `            if delta_x != 0 || delta_y != 0 {`
116. `                state_flags |= changing;`
117. `                element_ptr.next.location.x += delta_x;`
118. `                element_ptr.next.location.y += delta_y;`
119. `            }`
120. `        }`
121. `        if colliding_element(element_ptr) {`
122. `            init_intersect_end_point(element_ptr); // FFI:`
123. `        }`
124. `        if (state_flags & finite_life) != 0 {`
125. `            // [VALIDATE] decrement only when finite_life`
126. `            element_ptr.life_span -= 1;`
127. `        }`
128. `    }`
129. `    element_ptr.state_flags = (state_flags & !(post_process | collision)) | pre_process;`
130. `}`

## post_process()
131. `fn post_process(element_ptr: &mut Element) { // Phase1: Element`
132. `    if let Some(postprocess_func) = element_ptr.postprocess_func {`
133. `        postprocess_func(element_ptr); // FFI: callback`
134. `    }`
135. `    element_ptr.current = element_ptr.next; // commit`
136. `    if colliding_element(element_ptr) {`
137. `        init_intersect_start_point(element_ptr); // FFI:`
138. `        init_intersect_end_point(element_ptr); // FFI:`
139. `    }`
140. `    // DEFY_PHYSICS clearing is handled by collision/physics callbacks before this commit point`
141. `    element_ptr.state_flags = (element_ptr.state_flags & !(pre_process | changing | appearing)) | post_process;`
142. `}`

## process_collisions()
143. `fn process_collisions(`
144. `    mut h_succ_element: Helement,`
145. `    element_ptr: &mut Element, // Phase1: Element`
146. `    min_time: TimeValue, // Phase1: TIME_VALUE`
147. `    process_flags: ElementFlags,`
148. `) -> ElementFlags {`
149. `    let mut h_test_element: Helement;`
150. `    while { h_test_element = h_succ_element; h_test_element != 0 } {`
151. `        let mut test_element_ptr: *mut Element = null_mut();`
152. `        lock_element(h_test_element, &mut test_element_ptr); // FFI:`
153. `        // [VALIDATE] preprocess-on-demand for this pass flavor`
154. `        if ((*test_element_ptr).state_flags & process_flags) == 0 {`
155. `            pre_process(&mut *test_element_ptr);`
156. `        }`
157. `        h_succ_element = get_succ_element(test_element_ptr); // FFI:`
158. `        if test_element_ptr == (element_ptr as *mut Element) {`
159. `            unlock_element(h_test_element); // FFI:`
160. `            continue;`
161. `        }`
162. `        if collision_possible(test_element_ptr, element_ptr) { // FFI:`
163. `            let mut state_flags = element_ptr.state_flags;`
164. `            let mut test_state_flags = (*test_element_ptr).state_flags;`
165. `            let mut time_val: TimeValue;`
166. `            // APPEARING + FINITE_LIFE prefilter`
167. `            if (((state_flags | test_state_flags) & finite_life) != 0)`
168. `                && (((state_flags & appearing) != 0 && element_ptr.life_span > 1)`
169. `                    || ((test_state_flags & appearing) != 0 && (*test_element_ptr).life_span > 1)) {`
170. `                time_val = 0;`
171. `            } else {`
172. `                // stuck-overlap sentinel loop: DrawablesIntersect == 1`
173. `                while {`
174. `                    time_val = drawables_intersect(&element_ptr.intersect_control, &(*test_element_ptr).intersect_control, min_time); // FFI:`
175. `                    time_val == 1 && ((state_flags | test_state_flags) & finite_life) == 0`
176. `                } {`
177. `                    #[cfg(DEBUG_PROCESS)]`
178. `                    { log_add(log_debug, "BAD NEWS ..."); // FFI: DEBUG_PROCESS branch }`
179. `                    if (state_flags & collision) != 0 {`
180. `                        init_intersect_end_point(&mut *test_element_ptr); // FFI:`
181. `                        (*test_element_ptr).intersect_control.intersect_stamp.origin = (*test_element_ptr).intersect_control.end_point;`
182. `                        time_val = drawables_intersect(&element_ptr.intersect_control, &(*test_element_ptr).intersect_control, 1); // FFI:`
183. `                        init_intersect_start_point(&mut *test_element_ptr); // FFI:`
184. `                    }`
185. `                    if time_val == 1 {`
186. `                        let cur_frame = element_ptr.current.image.frame;`
187. `                        let next_frame = element_ptr.next.image.frame;`
188. `                        let test_cur_frame = (*test_element_ptr).current.image.frame;`
189. `                        let test_next_frame = (*test_element_ptr).next.image.frame;`
190. `                        if next_frame == cur_frame && test_next_frame == test_cur_frame {`
191. `                            if (test_state_flags & appearing) != 0 {`
192. `                                do_damage(&mut *test_element_ptr, (*test_element_ptr).hit_points); // FFI:`
193. `                                if !(*test_element_ptr).p_parent.is_null() { untarget(&mut *test_element_ptr); }`
194. `                                (*test_element_ptr).state_flags |= collision | disappearing;`
195. `                                if let Some(death) = (*test_element_ptr).death_func { death(&mut *test_element_ptr); }`
196. `                            }`
197. `                            if (state_flags & appearing) != 0 {`
198. `                                do_damage(element_ptr, element_ptr.hit_points); // FFI:`
199. `                                if !element_ptr.p_parent.is_null() { untarget(element_ptr); }`
200. `                                element_ptr.state_flags |= collision | disappearing;`
201. `                                if let Some(death) = element_ptr.death_func { death(element_ptr); }`
202. `                                unlock_element(h_test_element); // FFI:`
203. `                                return collision;`
204. `                            }`
205. `                            time_val = 0;`
206. `                        } else {`
207. `                            // normalization/fallback sequence for both elements`
208. `                            if get_frame_index(cur_frame) != get_frame_index(next_frame) { // FFI:`
209. `                                element_ptr.next.image.frame = set_equ_frame_index(next_frame, cur_frame); // FFI:`
210. `                            } else if next_frame != cur_frame {`
211. `                                element_ptr.next.image = element_ptr.current.image;`
212. `                                if element_ptr.life_span > normal_life { element_ptr.life_span = normal_life; }`
213. `                            }`
214. `                            if get_frame_index(test_cur_frame) != get_frame_index(test_next_frame) { // FFI:`
215. `                                (*test_element_ptr).next.image.frame = set_equ_frame_index(test_next_frame, test_cur_frame); // FFI:`
216. `                            } else if test_next_frame != test_cur_frame {`
217. `                                (*test_element_ptr).next.image = (*test_element_ptr).current.image;`
218. `                                if (*test_element_ptr).life_span > normal_life { (*test_element_ptr).life_span = normal_life; }`
219. `                            }`
220. `                            init_intersect_start_point(element_ptr); init_intersect_end_point(element_ptr); init_intersect_frame(element_ptr); // FFI:`
221. `                            if (state_flags & player_ship) != 0 {`
222. `                                let mut starship_ptr: *mut Starship = null_mut(); // Phase1: STARSHIP`
223. `                                get_element_star_ship(element_ptr, &mut starship_ptr); // FFI:`
224. `                                (*starship_ptr).ship_facing = get_frame_index(element_ptr.next.image.frame); // FFI:`
225. `                            }`
226. `                            init_intersect_start_point(&mut *test_element_ptr); init_intersect_end_point(&mut *test_element_ptr); init_intersect_frame(&mut *test_element_ptr); // FFI:`
227. `                            if (test_state_flags & player_ship) != 0 {`
228. `                                let mut starship_ptr: *mut Starship = null_mut();`
229. `                                get_element_star_ship(&mut *test_element_ptr, &mut starship_ptr); // FFI:`
230. `                                (*starship_ptr).ship_facing = get_frame_index((*test_element_ptr).next.image.frame); // FFI:`
231. `                            }`
232. `                        }`
233. `                    }`
234. `                    if time_val == 0 {`
235. `                        init_intersect_end_point(element_ptr); // FFI:`
236. `                        init_intersect_end_point(&mut *test_element_ptr); // FFI:`
237. `                        break;`
238. `                    }`
239. `                }`
240. `            }`
241. `            if time_val > 0 {`
242. `                let save_pt = element_ptr.intersect_control.end_point; // Phase1: POINT`
243. `                let test_save_pt = (*test_element_ptr).intersect_control.end_point;`
244. `                #[cfg(DEBUG_PROCESS)]`
245. `                { log_add(log_debug, "... at time"); // FFI: DEBUG_PROCESS branch }`
246. `                init_intersect_end_point(element_ptr); // FFI:`
247. `                init_intersect_end_point(&mut *test_element_ptr); // FFI:`
248. `                // recursive structure: pre-collision sub-interval resolution`
249. `                if time_val == 1`
250. `                    || (((state_flags & collision) != 0`
251. `                        || process_collisions(h_succ_element, element_ptr, time_val - 1, process_flags) == 0)`
252. `                        && ((test_state_flags & collision) != 0`
253. `                            || process_collisions(`
254. `                                if ((*test_element_ptr).state_flags & appearing) == 0 { get_succ_element(element_ptr as *mut Element) } else { get_head_element() },`
255. `                                &mut *test_element_ptr,`
256. `                                time_val - 1,`
257. `                                process_flags,
258. `                            ) == 0)) {`
259. `                    state_flags = element_ptr.state_flags;`
260. `                    test_state_flags = (*test_element_ptr).state_flags;`
261. `                    #[cfg(DEBUG_PROCESS)]`
262. `                    { log_add(log_debug, "PROCESSING ..."); // FFI: DEBUG_PROCESS branch }`
263. `                    // dispatch ordering: player ship side first when test is PLAYER_SHIP`
264. `                    if (test_state_flags & player_ship) != 0 {`
265. `                        (*test_element_ptr).collision_func.unwrap()(&mut *test_element_ptr, &test_save_pt, element_ptr, &save_pt); // FFI: callback`
266. `                        element_ptr.collision_func.unwrap()(element_ptr, &save_pt, &mut *test_element_ptr, &test_save_pt); // FFI: callback`
267. `                    } else {`
268. `                        element_ptr.collision_func.unwrap()(element_ptr, &save_pt, &mut *test_element_ptr, &test_save_pt); // FFI: callback`
269. `                        (*test_element_ptr).collision_func.unwrap()(&mut *test_element_ptr, &test_save_pt, element_ptr, &save_pt); // FFI: callback`
270. `                    }`
271. `                    if ((*test_element_ptr).state_flags & collision) != 0 && (test_state_flags & collision) == 0 {`
272. `                        (*test_element_ptr).intersect_control.intersect_stamp.origin = test_save_pt;`
273. `                        (*test_element_ptr).next.location.x = display_to_world(test_save_pt.x); // FFI:`
274. `                        (*test_element_ptr).next.location.y = display_to_world(test_save_pt.y); // FFI:`
275. `                        init_intersect_end_point(&mut *test_element_ptr); // FFI:`
276. `                    }`
277. `                    if (element_ptr.state_flags & collision) != 0 {`
278. `                        if (state_flags & collision) == 0 {`
279. `                            element_ptr.intersect_control.intersect_stamp.origin = save_pt;`
280. `                            element_ptr.next.location.x = display_to_world(save_pt.x); // FFI:`
281. `                            element_ptr.next.location.y = display_to_world(save_pt.y); // FFI:`
282. `                            init_intersect_end_point(element_ptr); // FFI:`
283. `                            if (state_flags & finite_life) == 0 && (test_state_flags & finite_life) == 0 {`
284. `                                collide(element_ptr, &mut *test_element_ptr); // FFI: bounce/physics`
285. `                                // post-bounce rescans`
286. `                                process_collisions(get_head_element(), element_ptr, max_time_value, process_flags);`
287. `                                process_collisions(get_head_element(), &mut *test_element_ptr, max_time_value, process_flags);`
288. `                            }`
289. `                        }`
290. `                        unlock_element(h_test_element); // FFI:`
291. `                        return collision;`
292. `                    }`
293. `                    if !colliding_element(element_ptr) {`
294. `                        element_ptr.state_flags |= collision;`
295. `                        unlock_element(h_test_element); // FFI:`
296. `                        return collision;`
297. `                    }`
298. `                }`
299. `            }`
300. `        }`
301. `        unlock_element(h_test_element); // FFI:`
302. `    }`
303. `    return element_ptr.state_flags & collision;`
304. `}`

## compile-time branch variants captured
305. `[VALIDATE] KDEBUG variant:`
306. `- In this requested function set (alloc/free/setup/untarget/remove/pre/post/process_collisions), no KDEBUG branches exist in the C body.`
307. `- KDEBUG appears in neighboring functions (e.g., CalcReduction/CalcView/PreProcessQueue), not in the covered set.`
308. `[VALIDATE] DEBUG_PROCESS variant in process_collisions:`
309. `- Logs "BAD NEWS ..." inside stuck-overlap sentinel loop.`
310. `- Logs collision candidate with time value before recursive gate.`
311. `- Logs "PROCESSING ..." before callback dispatch.`