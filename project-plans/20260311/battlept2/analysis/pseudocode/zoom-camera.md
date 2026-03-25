001. # zoom_camera pseudocode (process.c 736-1108)
002. 
003. ## calc_reduction(min_reduction_input, ships_alive) -> (view_state, min_reduction_out, zoom_out_updated)
004. 001. [phase1] purpose: compute zoom level target from ship extents and smooth/step hysteresis.
005. 002. [ffi] mirrors C behavior for `optMeleeScale == TFB_SCALE_STEP` and continuous modes.
006. 003. if ships_alive <= 1:
007. 004.     if opt_melee_scale == tfb_scale_step:
008. 005.         min_reduction = 0
009. 006.     else:
010. 007.         min_reduction = 1 << zoom_shift
011. 008.     [validate] single-ship fallback matches C path before CalcView return.
012. 009.     return calc_view(origin, min_reduction, pscroll_x, pscroll_y, ships_alive)
013. 010. end_if
014. 011. 
015. 012. # upstream pre_process has already scanned active ships and computed:
016. 013. #   min_x, max_x, min_y, max_y and candidate reductions from extents.
017. 014. # this function focuses on hysteresis + interpolation update of global zoom_out.
018. 015. 
019. 016. if opt_melee_scale == tfb_scale_step:
020. 017.     # 3-level hysteresis in integer zoom levels.
021. 018.     # min_reduction is an integer desired zoom step (0..max_vis_reduction).
022. 019.     # zoom_out tracks current step with lag to prevent jitter.
023. 020.     if min_reduction > zoom_out:
024. 021.         # need to zoom further out immediately when objects exceed current view.
025. 022.         zoom_out = min_reduction
026. 023.         view_state = view_change
027. 024.     else:
028. 025.         delta = zoom_out - min_reduction
029. 026.         if delta >= 3:
030. 027.             # hysteresis threshold: only zoom back in when 3+ levels inside margin.
031. 028.             zoom_out = zoom_out - 1
032. 029.             view_state = view_change
033. 030.         else if delta >= 1:
034. 031.             # inside hysteresis deadband; keep view steady.
035. 032.             view_state = view_wait
036. 033.         else:
037. 034.             # exactly matched.
038. 035.             view_state = view_stable
039. 036.         end_if
040. 037.     end_if
041. 038. else:
042. 039.     # continuous smooth interpolation mode (fixed-point by zoom_shift bits).
043. 040.     # min_reduction uses same fixed-point scale as zoom_out.
044. 041.     if min_reduction > zoom_out:
045. 042.         # zooming out responds quickly to preserve visibility.
046. 043.         zoom_out = min_reduction
047. 044.         view_state = view_change
048. 045.     else:
049. 046.         diff = zoom_out - min_reduction
050. 047.         if diff > 0:
051. 048.             # smooth zoom-in toward target; C does gradual decrement (no snap).
052. 049.             # exact decrement step is fixed-point small-step easing.
053. 050.             zoom_out = zoom_out - smooth_step_toward_target(diff)
054. 051.             if zoom_out != previous_zoom_out:
055. 052.                 view_state = view_change
056. 053.             else:
057. 054.                 view_state = view_wait
058. 055.             end_if
059. 056.         else:
060. 057.             view_state = view_stable
061. 058.         end_if
062. 059.     end_if
063. 060. end_if
064. 061. 
065. 062. [validate] clamp zoom_out to configured max (opt_max_zoom_out / max_vis_reduction domain).
066. 063. return calc_view(origin, min_reduction, pscroll_x, pscroll_y, ships_alive)
067. 
068. ## calc_view(...) / calc_zoom_stuff macro
069. 001. [phase1] role: derive camera origin (`space_org`) and render scale from zoom state.
070. 002. inputs include tracked ship bounds/center and current zoom_out.
071. 003. 
072. 004. # camera midpoint
073. 005. mid_x = (min_x + max_x) / 2
074. 006. mid_y = (min_y + max_y) / 2
075. 007. 
076. 008. # single-ship clamping fallback when only one ship remains
077. 009. if ships_alive <= 1:
078. 010.     mid_x = tracked_ship_x_or_origin_x
079. 011.     mid_y = tracked_ship_y_or_origin_y
080. 012. end_if
081. 013. 
082. 014. # aspect-ratio normalization:
083. 015. # choose reduction required by x-span and y-span after scaling to viewport aspect.
084. 016. req_x = reduction_needed_for_width(max_x - min_x)
085. 017. req_y = reduction_needed_for_height(max_y - min_y)
086. 018. min_reduction = max(req_x, req_y)
087. 019. 
088. 020. # transition output for renderer
089. 021. if opt_melee_scale == tfb_scale_step:
090. 022.     zoom_index = zoom_out
091. 023.     gscale = gscale_identity
092. 024. else:
093. 025.     # calc_zoom_stuff(index*, scale*):
094. 026.     # decompose fixed-point zoom_out into discrete sprite bank index + subscale.
095. 027.     zoom_index = integer_zoom_level_from(zoom_out)
096. 028.     gscale = interpolation_scale_from_fractional_zoom(zoom_out)
097. 029.     [validate] zoom_index in available farray range; gscale valid for SetGraphicScale.
098. 030. end_if
099. 031. 
100. 032. # compute world origin so midpoint maps to screen center
101. 033. view_half_w_world = viewport_half_width_in_world_units(zoom_out)
102. 034. view_half_h_world = viewport_half_height_in_world_units(zoom_out)
103. 035. space_org.x = wrap_x(mid_x - view_half_w_world)
104. 036. space_org.y = wrap_y(mid_y - view_half_h_world)
105. 037. 
106. 038. # scroll deltas exported for elements not post-processed this tick
107. 039. pscroll_x = old_space_org.x - space_org.x
108. 040. pscroll_y = old_space_org.y - space_org.y
109. 041. 
110. 042. return view_state
111. 
112. ## insert_prim(p_links, prim_index, insert_before_index)
113. 001. [phase1] insert primitive into doubly-linked display ordering list.
114. 002. if insert_before_index == end_of_list:
115. 003.     tail = succ_link(*p_links)
116. 004.     if tail == end_of_list:
117. 005.         *p_links = make_links(prim_index, prim_index)
118. 006.     else:
119. 007.         *p_links = make_links(pred_link(*p_links), prim_index)
120. 008.     end_if
121. 009. else:
122. 010.     links_at_insert = get_prim_links(display_array[insert_before_index])
123. 011.     if insert_before_index != pred_link(*p_links):
124. 012.         prev = pred_link(links_at_insert)
125. 013.     else:
126. 014.         prev = end_of_list
127. 015.         *p_links = make_links(prim_index, succ_link(*p_links))
128. 016.     end_if
129. 017.     set_prim_links(display_array[insert_before_index], prim_index, succ_link(links_at_insert))
130. 018. end_if
131. 019. if prev != end_of_list:
132. 020.     prev_links = get_prim_links(display_array[prev])
133. 021.     set_prim_links(display_array[prev], pred_link(prev_links), prim_index)
134. 022. end_if
135. 023. set_prim_links(display_array[prim_index], prev, insert_before_index)
136. 024. [validate] list remains well-formed (head/tail + predecessor/successor consistency).
137. 
138. ## calc_display_coord(c, org_c, reduction)
139. 001. if opt_melee_scale == tfb_scale_step:
140. 002.     # legacy shift scaling
141. 003.     return (c - org_c) >> reduction
142. 004. else:
143. 005.     # fixed-point continuous scaling
144. 006.     return ((c - org_c) << zoom_shift) / reduction
145. 007. end_if
146. 008. [validate] reduction nonzero in continuous mode.
147. 
148. ## pre_process_queue(pscroll_x*, pscroll_y*) -> view_state
149. 001. iterate element list from head.
150. 002. for each element:
151. 003.     lock element
152. 004.     if element has pre_process flag:
153. 005.         call pre_process(element)
154. 006.         if element collidable and collision flag not set:
155. 007.             process_collisions(head_element, element, max_time_value, pre_process)
156. 008.         end_if
157. 009.     end_if
158. 010.     gather ship position extents/alive-count used by calc_reduction/calc_view
159. 011.     unlock element
160. 012. end_for
161. 013. compute min_reduction via extent/aspect rules
162. 014. apply step-3-level hysteresis or continuous interpolation
163. 015. output scroll deltas via calc_view
164. 016. return view_state
165. 
166. ## post_process_queue(view_state, scroll_x, scroll_y)
167. 001. if opt_melee_scale == tfb_scale_step:
168. 002.     reduction = zoom_out + one_shift
169. 003. else:
170. 004.     reduction = zoom_out << one_shift
171. 005. end_if
172. 006. 
173. 007. h_element = get_head_element()
174. 008. while h_element != 0:
175. 009.     lock element
176. 010.     state_flags = element.state_flags
177. 011. 
178. 012.     if state_flags has pre_process:
179. 013.         if not collision: clear defy_physics else clear collision
180. 014.         if state_flags has post_process:
181. 015.             delta = (0, 0)
182. 016.         else:
183. 017.             delta = (scroll_x, scroll_y)
184. 018.         end_if
185. 019.     else:
186. 020.         # cascading preprocess for newly added elements
187. 021.         h_post = h_element
188. 022.         do:
189. 023.             lock post_element
190. 024.             if post_element lacks pre_process: pre_process(post_element)
191. 025.             h_next = get_succ_element(post_element)
192. 026.             if post_element collidable and not collision:
193. 027.                 process_collisions(get_head_element(), post_element, max_time_value, pre_process | post_process)
194. 028.             end_if
195. 029.             unlock post_element
196. 030.             h_post = h_next
197. 031.         while h_post != 0
198. 032.         scroll_x = 0; scroll_y = 0
199. 033.         delta = (0, 0)  # newly added are already adjusted
200. 034.         state_flags = element.state_flags
201. 035.     end_if
202. 036. 
203. 037.     if state_flags has disappearing:
204. 038.         h_next_element = get_succ_element(element)
205. 039.         unlock element
206. 040.         remove_element(h_element)
207. 041.         free_element(h_element)
208. 042.     else:
209. 043.         obj_type = get_prim_type(display_array[element.prim_index])
210. 044.         if view_state != view_stable or state_flags has appearing/changing:
211. 045.             if obj_type == line_prim:
212. 046.                 dx = element.next.location.x - element.current.location.x
213. 047.                 dy = element.next.location.y - element.current.location.y
214. 048.                 next = wrap(current + delta)
215. 049.                 display.line.first = calc_display_coord(next, space_org, reduction)
216. 050.                 next += (dx, dy)
217. 051.                 display.line.second = calc_display_coord(next, space_org, reduction)
218. 052.             else:
219. 053.                 next = wrap(element.next.location + delta)
220. 054.                 display.point = calc_display_coord(next, space_org, reduction)
221. 055.                 if obj_type is stamp or stampfill:
222. 056.                     if view_state == view_change or state_flags has appearing/changing:
223. 057.                         if opt_melee_scale == tfb_scale_step:
224. 058.                             index = zoom_out; scale = gscale_identity
225. 059.                         else:
226. 060.                             calc_zoom_stuff(&index, &scale)
227. 061.                         end_if
228. 062.                         element.next.image.frame = set_equ_frame_index(element.next.image.farray[index], element.next.image.frame)
229. 063.                         if opt_melee_scale == tfb_scale_trilinear and index < 2 and scale != gscale_identity:
230. 064.                             mmframe = set_equ_frame_index(element.next.image.farray[index + 1], frame)
231. 065.                             if frame and mmframe:
232. 066.                                 tfb_drawscreen_set_mipmap(frame.image, mmframe.image, mm_hotspot)
233. 067.                             end_if
234. 068.                         end_if
235. 069.                     end_if
236. 070.                     display.stamp.frame = element.next.image.frame
237. 071.                 end_if
238. 072.             end_if
239. 073.             element.next.location = next
240. 074.         end_if
241. 075. 
242. 076.         post_process(element)
243. 077.         if obj_type < num_prims:
244. 078.             insert_prim(&display_links, element.prim_index, end_of_list)
245. 079.         end_if
246. 080.         h_next_element = get_succ_element(element)
247. 081.         unlock element
248. 082.     end_if
249. 083. 
250. 084.     h_element = h_next_element
251. 085. end_while
252. 086. [validate] no locked handles remain; removed elements are not reused.
253. 
254. ## init_display_list()
255. 001. if opt_melee_scale == tfb_scale_step:
256. 002.     zoom_out = max_vis_reduction + 1
257. 003.     opt_max_zoom_out = max_vis_reduction
258. 004. else:
259. 005.     zoom_out = max_zoom_out + (1 << zoom_shift)
260. 006.     opt_max_zoom_out = max_zoom_out
261. 007. end_if
262. 008. reinit_queue(disp_q)
263. 009. for i in 0..max_display_prims-1:
264. 010.     set_prim_links(display_array[i], end_of_list, i + 1)
265. 011. end_for
266. 012. set_prim_links(display_array[last], end_of_list, end_of_list)
267. 013. display_free_list = 0
268. 014. display_links = make_links(end_of_list, end_of_list)
269. 
270. ## redraw_queue(clear)
271. 001. set_context(status_context)
272. 002. view_state = pre_process_queue(&scroll_x, &scroll_y)
273. 003. post_process_queue(view_state, scroll_x, scroll_y)
274. 004. if opt_stereo_sfx: update_sound_positions()
275. 005. 
276. 006. set_context(space_context)
277. 007. if activity allows drawing:
278. 008.     skip_frames = hi_byte(nth_frame)
279. 009.     if skip_frames enabled and frame_gate_passed:
280. 010.         nth_frame += skip_frames
281. 011.         if clear: clear_drawable()
282. 012.         if opt_melee_scale != tfb_scale_step:
283. 013.             calc_zoom_stuff(&index, &scale)
284. 014.             set_graphic_scale(scale)
285. 015.         end_if
286. 016.         draw_batch(display_array, display_links, 0)
287. 017.         set_graphic_scale(0)
288. 018.     end_if
289. 019.     flush_sounds()
290. 020. else:
291. 021.     process_sound(~0, null)
292. 022.     flush_sounds()
293. 023. end_if
294. 024. display_links = make_links(end_of_list, end_of_list)
295. 
296. ## init_kernel()
297. 001. [phase1] initialize runtime graphics kernel state before battle loop.
298. 002. initialize display queue + display list structures (calls init_display_list).
299. 003. initialize contexts/resources required by redraw path (status_context/space_context, draw batch state).
300. 004. reset frame skip counter (`nth_frame`) and zoom/camera globals (`space_org`, `zoom_out`).
301. 005. ensure primitive freelist and link sentinels are valid.
302. 006. [ffi] keep API-compatible entry point for platform bindings.
303. 007. [validate] after init: display_links empty, display_free_list seeded, zoom defaults match scaling mode.
