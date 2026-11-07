//! Incremental UTF-8 matching and typed callbacks owned by the map visit.
use super::*;
use crate::strings::unicode::{case_prefix_matches, count_utf8_bytes, split_utf8_prefix};
use crate::ui::getchar::{edit_text, TextEntryCallbacks, TextEntryView};

#[derive(Default)]
struct Query {
    prefix: Option<Vec<u8>>,
    cluster: Option<Vec<u8>>,
    cluster_pos: usize,
}
impl Query {
    fn new(text: &[u8]) -> Self {
        let start = text
            .iter()
            .position(|&byte| byte != b' ')
            .unwrap_or(text.len());
        if start == text.len() {
            return Self::default();
        }
        let end = text[start..]
            .iter()
            .position(|&byte| byte == b' ')
            .map_or(text.len(), |index| start + index);
        let next = text[end..]
            .iter()
            .position(|&byte| byte != b' ')
            .map_or(text.len(), |index| end + index);
        if next == text.len() {
            Self {
                prefix: None,
                cluster: Some(text[start..].to_vec()),
                cluster_pos: count_utf8_bytes(&text[..start]),
            }
        } else {
            Self {
                prefix: Some(text[start..end].to_vec()),
                cluster: Some(text[next..].to_vec()),
                cluster_pos: count_utf8_bytes(&text[..next]),
            }
        }
    }
}

struct SearchModel {
    text: CString,
    query: Query,
    first: Option<usize>,
    current: Option<usize>,
    last: Option<usize>,
    single_cluster: bool,
    single_match: bool,
    sorted: Vec<usize>,
}
impl SearchModel {
    fn new(sorted: Vec<usize>) -> Self {
        Self {
            text: c"".into(),
            query: Query::default(),
            first: None,
            current: Some(0),
            last: None,
            single_cluster: false,
            single_match: false,
            sorted,
        }
    }

    fn find<'a>(
        &self,
        from: usize,
        within: bool,
        stars: &[CStarDesc],
        cluster_name: &impl Fn(u8) -> &'a CStr,
        full_name: &impl Fn(&CStarDesc) -> CString,
    ) -> Option<usize> {
        let cluster = self.query.cluster.as_deref()?;
        let cluster_length = count_utf8_bytes(cluster);
        let mut index = from;
        while index < stars.len() {
            let star = &stars[self.sorted[index]];
            let name = cluster_name(star.postfix).to_bytes();
            if cluster_length > count_utf8_bytes(name)
                || !case_prefix_matches(cluster, name, cluster_length)
            {
                index = self.skip_cluster(stars, index);
                continue;
            }
            if self.query.prefix.is_some() && star.prefix == 0 {
                index += 1;
                continue;
            }
            if within {
                break;
            }
            let Some(prefix) = &self.query.prefix else {
                if index == 0 || star.postfix != stars[self.sorted[index - 1]].postfix {
                    break;
                }
                index = self.skip_cluster(stars, index);
                continue;
            };
            let name = full_name(star);
            let prefix_length = count_utf8_bytes(prefix);
            if prefix_length <= count_utf8_bytes(name.to_bytes())
                && case_prefix_matches(prefix, name.to_bytes(), prefix_length)
            {
                break;
            }
            index += 1;
        }
        (index < stars.len()).then_some(index)
    }

    fn skip_cluster(&self, stars: &[CStarDesc], mut index: usize) -> usize {
        let postfix = stars[self.sorted[index]].postfix;
        index += 1;
        while index < self.sorted.len() && stars[self.sorted[index]].postfix == postfix {
            index += 1;
        }
        index
    }

    fn advance<'a>(
        &mut self,
        reset: bool,
        stars: &[CStarDesc],
        cluster_name: impl Fn(u8) -> &'a CStr,
        full_name: impl Fn(&CStarDesc) -> CString,
    ) {
        if reset {
            self.first = None;
        }
        if self.first.is_none() {
            self.current = None;
            self.last = None;
            self.single_cluster = false;
            self.single_match = false;
            self.query = Query::new(self.text.to_bytes());
        }
        self.current = self.find(
            self.current.map_or(0, |index| index + 1),
            self.single_cluster,
            stars,
            &cluster_name,
            &full_name,
        );
        if self.first.is_none() {
            self.first = self.current;
        }
        if self.current.is_some() {
            self.last = self.current;
        } else {
            self.current = self.first;
            if self.first == self.last && self.first.is_some() {
                if self.query.prefix.is_none() {
                    self.single_cluster = true;
                } else {
                    self.single_match = true;
                }
            }
        }
    }
}

fn sorted_stars<'a>(stars: &[CStarDesc], cluster: impl Fn(u8) -> &'a CStr) -> Vec<usize> {
    let mut sorted: Vec<_> = (0..stars.len()).collect();
    sorted.sort_by(|&first, &second| {
        let first = &stars[first];
        let second = &stars[second];
        if first.postfix != second.postfix {
            cluster(first.postfix)
                .to_bytes()
                .cmp(cluster(second.postfix).to_bytes())
        } else {
            first.prefix.cmp(&second.prefix)
        }
    });
    sorted
}

struct Search<'a> {
    map: &'a mut StarMap,
    model: SearchModel,
}
impl Search<'_> {
    fn advance(&mut self, cx: &Cx<'_>, reset: bool) {
        self.model.advance(
            reset,
            &cx.world.catalog()[..NUM_SOLAR_SYSTEMS],
            |postfix| cx.kernel.game_text(usize::from(postfix)),
            |star| {
                super::super::starmap::naming::cluster_name(
                    cx.kernel,
                    star,
                    crate::navigation::abi::STAR_NUMBER_BASE as usize,
                )
            },
        );
    }

    fn draw_match(&mut self, entry: &TextEntryView<'_>, cx: &Cx<'_>) {
        let index = self
            .model
            .current
            .unwrap_or_else(|| panic!("drawing requires a matched star"));
        let star = &cx.world.catalog()[self.model.sorted[index]];
        let (text, cursor, extra, flags) = if self.model.single_cluster || self.model.single_match {
            (
                super::super::starmap::naming::cluster_name(
                    cx.kernel,
                    star,
                    crate::navigation::abi::STAR_NUMBER_BASE as usize,
                ),
                -1,
                -1,
                1,
            )
        } else {
            let cluster = self
                .model
                .query
                .cluster
                .as_deref()
                .unwrap_or_else(|| panic!("match requires a cluster"));
            let extra = (self.model.query.cluster_pos + count_utf8_bytes(cluster)) as i16;
            let (prefix, _) =
                split_utf8_prefix(self.model.text.to_bytes(), self.model.query.cluster_pos);
            let mut text = prefix.to_vec();
            text.extend_from_slice(cx.kernel.game_text(usize::from(star.postfix)).to_bytes());
            (
                CString::new(text).unwrap_or_else(|error| panic!("invalid matched label: {error}")),
                entry.cursor as i16,
                extra,
                2 | if entry.joystick { 4 } else { 0 },
            )
        };
        crate::ui::sis::draw_message(
            cx.kernel,
            self.map.dimensions().0,
            &text,
            cursor,
            extra,
            flags,
        );
        crate::ui::sis::draw_hyper_coords(
            cx.kernel,
            self.map.dimensions().0,
            self.map.state.cursor,
        );
    }

    fn move_match(&mut self, entry: &TextEntryView<'_>, cx: &Cx<'_>) {
        let index = self
            .model
            .current
            .unwrap_or_else(|| panic!("movement requires a matched star"));
        let point = cx.world.catalog()[self.model.sorted[index]].star_pt;
        self.map.move_cursor(cx.world, 0, 0, Some(point));
        self.draw_match(entry, cx);
        self.map.fuel(cx.kernel, cx.world);
    }
}
impl TextEntryCallbacks for Search<'_> {
    fn changed(
        &mut self,
        entry: TextEntryView<'_>,
        cx: &mut Cx<'_>,
        _input: &InputSession,
    ) -> bool {
        if entry.text != self.model.text.as_c_str() {
            self.model.text = entry.text.into();
            self.advance(cx, true);
        }
        if self.model.current.is_none() {
            let flags =
                if self.model.text.is_empty() { 1 } else { 2 } | if entry.joystick { 4 } else { 0 };
            crate::ui::sis::draw_message(
                cx.kernel,
                self.map.dimensions().0,
                &self.model.text,
                entry.cursor as i16,
                -1,
                flags,
            )
        } else {
            self.move_match(&entry, cx);
            true
        }
    }

    fn frame(
        &mut self,
        entry: TextEntryView<'_>,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        input: &InputSession,
    ) -> bool {
        self.model.text = entry.text.into();
        if input.pulsed().menu[22] != 0 {
            self.advance(cx, false);
            if self.model.current.is_none() {
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
                return true;
            }
            self.move_match(&entry, cx);
        }
        self.map.observe(io);
        self.map.flash(cx.kernel);
        cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
        true
    }
}

pub(super) fn run(
    map: &mut StarMap,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
) -> bool {
    let sorted = sorted_stars(&cx.world.catalog()[..NUM_SOLAR_SYSTEMS], |postfix| {
        cx.kernel.game_text(usize::from(postfix))
    });
    let mut search = Search {
        map,
        model: SearchModel::new(sorted),
    };
    let mut buffer = [0; 256];
    crate::ui::sis::draw_message(cx.kernel, search.map.dimensions().0, c"", 0, 0, 1);
    input.set_menu_sounds(15, 16);
    input.configure_repeat(
        ONE_SECOND / 12,
        ONE_SECOND / 2,
        ONE_SECOND / 12,
        false,
        GetTimeCounter(),
    );
    let success = edit_text(cx, io, input, &mut buffer, 0, &mut search);
    let text = CStr::from_bytes_until_nul(&buffer)
        .unwrap_or_else(|error| panic!("search editor lost text terminator: {error}"));
    crate::ui::sis::draw_message(cx.kernel, search.map.dimensions().0, text, -1, -1, 2);
    success
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::input::InputRuntime;
    fn star(prefix: u8, postfix: u8) -> CStarDesc {
        CStarDesc {
            star_pt: POINT { x: 0, y: 0 },
            star_type: 0,
            index: 0,
            prefix,
            postfix,
        }
    }
    struct Catalog {
        stars: Vec<CStarDesc>,
        clusters: Vec<CString>,
        full: Vec<CString>,
    }
    impl Catalog {
        fn new() -> Self {
            Self {
                stars: vec![star(1, 0), star(2, 0), star(1, 1), star(2, 1), star(0, 2)],
                clusters: ["Centauri", "Ceti", "Sol"]
                    .map(|s| CString::new(s).unwrap())
                    .into(),
                full: [
                    "Alpha Centauri",
                    "Beta Centauri",
                    "Alpha Ceti",
                    "Beta Ceti",
                    "Sol",
                ]
                .map(|s| CString::new(s).unwrap())
                .into(),
            }
        }
        fn name(&self, star: &CStarDesc) -> CString {
            self.full[self
                .stars
                .iter()
                .position(|candidate| std::ptr::eq(candidate, star))
                .unwrap()]
            .clone()
        }
        fn advance(&self, model: &mut SearchModel, reset: bool) {
            model.advance(
                reset,
                &self.stars,
                |postfix| self.clusters[usize::from(postfix)].as_c_str(),
                |star| self.name(star),
            );
        }
    }
    #[test]
    fn split_preserves_ascii_spaces_and_utf8_character_positions() {
        for (text, prefix, cluster, position) in [
            ("", None, None, 0),
            ("   ", None, None, 0),
            ("Centauri", None, Some("Centauri"), 0),
            ("Alpha Centauri", Some("Alpha"), Some("Centauri"), 6),
            ("  α   Ceti", Some("α"), Some("Ceti"), 6),
            ("Alpha ", None, Some("Alpha "), 0),
            ("Alpha  Beta  ", Some("Alpha"), Some("Beta  "), 7),
        ] {
            let query = Query::new(text.as_bytes());
            assert_eq!(query.prefix.as_deref(), prefix.map(str::as_bytes), "{text}");
            assert_eq!(
                query.cluster.as_deref(),
                cluster.map(str::as_bytes),
                "{text}"
            );
            assert_eq!(query.cluster_pos, position, "{text}");
        }
    }
    #[test]
    fn matches_clusters_prefixes_singular_stars_and_case() {
        let catalog = Catalog::new();
        for (text, within, expected) in [
            ("ce", false, vec![0, 2]),
            ("ce", true, vec![0, 1, 2, 3]),
            ("b CE", false, vec![1, 3]),
            ("Alpha ce", false, vec![0, 2]),
            ("a Sol", false, vec![]),
            ("so", false, vec![4]),
            ("Centaurii", false, vec![]),
            ("", false, vec![]),
        ] {
            let mut model = SearchModel::new((0..5).collect());
            model.query = Query::new(text.as_bytes());
            let mut indices = Vec::new();
            let mut from = 0;
            while let Some(found) = model.find(
                from,
                within,
                &catalog.stars,
                &|postfix| catalog.clusters[usize::from(postfix)].as_c_str(),
                &|star| catalog.name(star),
            ) {
                indices.push(found);
                from = found + 1;
            }
            assert_eq!(indices, expected, "{text}");
        }
    }
    #[test]
    fn wraps_and_expands_a_single_cluster_then_resets_cache_on_edits() {
        let catalog = Catalog::new();
        let mut model = SearchModel::new((0..5).collect());
        model.text = c"Cent".into();
        catalog.advance(&mut model, true);
        assert_eq!(model.current, Some(0));
        assert!(!model.single_cluster);
        catalog.advance(&mut model, false);
        assert_eq!(model.current, Some(0));
        assert!(model.single_cluster);
        catalog.advance(&mut model, false);
        assert_eq!(model.current, Some(1));
        catalog.advance(&mut model, false);
        assert_eq!(model.current, Some(0));
        model.text = c"Beta Ceti".into();
        catalog.advance(&mut model, true);
        assert_eq!(model.current, Some(3));
        assert!(!model.single_cluster);
        catalog.advance(&mut model, false);
        assert_eq!(model.current, Some(3));
        assert!(model.single_match);
        model.text = c"absent".into();
        catalog.advance(&mut model, true);
        assert_eq!(model.current, None);
        assert_eq!(model.first, None);
        assert!(!model.single_match);
    }
    #[test]
    fn sorting_uses_localized_cluster_bytes_then_numeric_prefix() {
        let stars = [star(2, 0), star(1, 0), star(1, 1)];
        assert_eq!(
            sorted_stars(&stars, |postfix| [c"Zeta", c"Alpha"][usize::from(postfix)]),
            vec![2, 1, 0]
        );
    }
    #[test]
    #[serial_test::serial]
    fn callbacks_move_and_cycle_matches_then_cancel_through_the_real_editor() {
        if !crate::campaign_display_test::own_graphics_process(concat!(
            module_path!(),
            "::callbacks_move_and_cycle_matches_then_cancel_through_the_real_editor"
        )) {
            return;
        }
        super::super::owned_tests::with_map(|map, cx, driver| {
            let mut table = vec![b"Other\0".to_vec(); 450];
            table[10] = b"Centauri\0".to_vec();
            table[361] = b"Alpha\0".to_vec();
            table[362] = b"Beta\0".to_vec();
            cx.kernel
                .install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(table));
            let mut stars = *cx.world.catalog();
            for star in &mut stars[..NUM_SOLAR_SYSTEMS] {
                star.prefix = 0;
                star.postfix = 11;
            }
            stars[0].prefix = 1;
            stars[0].postfix = 10;
            stars[1].prefix = 2;
            stars[1].postfix = 10;
            cx.world.set_catalog(std::borrow::Cow::Owned(stars));
            cx.world.current_star = Some(crate::engine::StarId(0));
            let mut search = Search {
                map,
                model: SearchModel::new(sorted_stars(
                    &cx.world.catalog()[..NUM_SOLAR_SYSTEMS],
                    |postfix| cx.kernel.game_text(usize::from(postfix)),
                )),
            };
            let input = driver.input();
            let mut observer = crate::engine::observe::Unobserved;
            let mut io = Io {
                observer: &mut observer,
            };
            let view = |text| TextEntryView {
                text,
                cursor: 3,
                joystick: false,
            };
            assert!(search.changed(view(c"cen"), cx, input));
            assert_eq!(search.model.current, Some(0));
            assert_eq!(search.map.state.cursor, cx.world.catalog()[0].star_pt);
            driver.press(&[22]);
            assert!(search.frame(view(c"cen"), cx, &mut io, input));
            assert!(search.model.single_cluster);
            assert_eq!(search.model.current, Some(0));
            driver.press(&[]);
            driver.press(&[22]);
            assert!(search.frame(view(c"cen"), cx, &mut io, input));
            assert_eq!(search.model.current, Some(1));
            assert_eq!(search.map.state.cursor, cx.world.catalog()[1].star_pt);
            driver.press(&[]);
            driver.press(&[22]);
            assert!(search.frame(view(c"cen"), cx, &mut io, input));
            assert_eq!(search.model.current, Some(0));
            assert!(search.changed(view(c"Alpha Centauri"), cx, input));
            driver.press(&[]);
            driver.press(&[22]);
            assert!(search.frame(view(c"Alpha Centauri"), cx, &mut io, input));
            assert!(search.model.single_match);
            assert!(search.changed(view(c"absent"), cx, input));
            assert_eq!(search.model.current, None);
            driver.press(&[]);
            driver.press(&[22]);
            assert!(search.frame(view(c"absent"), cx, &mut io, input));
            assert!(search.changed(view(c""), cx, input));
            assert!(search.changed(
                TextEntryView {
                    text: c"missing",
                    cursor: 7,
                    joystick: true
                },
                cx,
                input
            ));
            drop(search);
            assert!(!run(map, cx, &mut io, input));
        });
    }
}
