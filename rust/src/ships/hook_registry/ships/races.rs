//! Stable callback entries.
#[cfg(not(test))]
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    [
        part0(),
        part1(),
        part2(),
        part3(),
        part4(),
        part5(),
        part6(),
        part7(),
        part8(),
        part9(),
        part10(),
        part11(),
        part12(),
        part13(),
        part14(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
#[cfg(not(test))]
fn part0() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::orz_native::engine::howitzer_collision as *const () as usize,
            "ships::races::orz_native::howitzer_collision",
        ),
        (
            crate::ships::races::orz_native::engine::intruder_preprocess as *const () as usize,
            "ships::races::orz_native::intruder_preprocess",
        ),
        (
            crate::ships::races::orz_native::engine::ion_preprocess as *const () as usize,
            "ships::races::orz_native::ion_preprocess",
        ),
        (
            crate::ships::races::orz_native::engine::marine_collision as *const () as usize,
            "ships::races::orz_native::marine_collision",
        ),
        (
            crate::ships::races::orz_native::engine::marine_preprocess as *const () as usize,
            "ships::races::orz_native::marine_preprocess",
        ),
        (
            crate::ships::races::orz_native::engine::orz_intelligence as *const () as usize,
            "ships::races::orz_native::orz_intelligence",
        ),
        (
            crate::ships::races::orz_native::engine::orz_preprocess as *const () as usize,
            "ships::races::orz_native::orz_preprocess",
        ),
        (
            crate::ships::races::orz_native::engine::turret_postprocess as *const () as usize,
            "ships::races::orz_native::turret_postprocess",
        ),
        (
            crate::ships::races::human_native::engine::human_intelligence as *const () as usize,
            "ships::races::human_native::human_intelligence",
        ),
    ]
}
#[cfg(not(test))]
fn part1() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::human_native::engine::human_postprocess as *const () as usize,
            "ships::races::human_native::human_postprocess",
        ),
        (
            crate::ships::races::human_native::engine::initialize_nuke as *const () as usize,
            "ships::races::human_native::initialize_nuke",
        ),
        (
            crate::ships::races::human_native::engine::nuke_preprocess as *const () as usize,
            "ships::races::human_native::nuke_preprocess",
        ),
        (
            crate::ships::races::human_native::engine::spawn_point_defense as *const () as usize,
            "ships::races::human_native::spawn_point_defense",
        ),
        (
            crate::ships::races::spathi_native::engine::spathi_intelligence as *const () as usize,
            "ships::races::spathi_native::spathi_intelligence",
        ),
        (
            crate::ships::races::spathi_native::engine::spathi_postprocess as *const () as usize,
            "ships::races::spathi_native::spathi_postprocess",
        ),
        (
            crate::ships::races::spathi_native::engine::spawn_butt_missile as *const () as usize,
            "ships::races::spathi_native::spawn_butt_missile",
        ),
        (
            crate::ships::races::pkunk_native::engine::animate as *const () as usize,
            "ships::races::pkunk_native::animate",
        ),
        (
            crate::ships::races::pkunk_native::engine::initialize_bug_missile as *const () as usize,
            "ships::races::pkunk_native::initialize_bug_missile",
        ),
        (
            crate::ships::races::pkunk_native::engine::intercept_pkunk_death as *const () as usize,
            "ships::races::pkunk_native::intercept_pkunk_death",
        ),
    ]
}
#[cfg(not(test))]
fn part2() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::pkunk_native::engine::new_pkunk as *const () as usize,
            "ships::races::pkunk_native::new_pkunk",
        ),
        (
            crate::ships::races::pkunk_native::engine::phoenix_transition as *const () as usize,
            "ships::races::pkunk_native::phoenix_transition",
        ),
        (
            crate::ships::races::pkunk_native::engine::pkunk_intelligence as *const () as usize,
            "ships::races::pkunk_native::pkunk_intelligence",
        ),
        (
            crate::ships::races::pkunk_native::engine::pkunk_postprocess as *const () as usize,
            "ships::races::pkunk_native::pkunk_postprocess",
        ),
        (
            crate::ships::races::pkunk_native::engine::pkunk_preprocess as *const () as usize,
            "ships::races::pkunk_native::pkunk_preprocess",
        ),
        (
            crate::ships::races::pkunk_native::engine::spawn_phoenix_trail as *const () as usize,
            "ships::races::pkunk_native::spawn_phoenix_trail",
        ),
        (
            crate::ships::races::urquan_native::engine::fighter_collision as *const () as usize,
            "ships::races::urquan_native::fighter_collision",
        ),
        (
            crate::ships::races::urquan_native::engine::fighter_postprocess as *const () as usize,
            "ships::races::urquan_native::fighter_postprocess",
        ),
        (
            crate::ships::races::urquan_native::engine::fighter_preprocess as *const () as usize,
            "ships::races::urquan_native::fighter_preprocess",
        ),
        (
            crate::ships::races::urquan_native::engine::initialize_fusion as *const () as usize,
            "ships::races::urquan_native::initialize_fusion",
        ),
    ]
}
#[cfg(not(test))]
fn part3() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::urquan_native::engine::spawn_fighters as *const () as usize,
            "ships::races::urquan_native::spawn_fighters",
        ),
        (
            crate::ships::races::urquan_native::engine::urquan_intelligence as *const () as usize,
            "ships::races::urquan_native::urquan_intelligence",
        ),
        (
            crate::ships::races::urquan_native::engine::urquan_postprocess as *const () as usize,
            "ships::races::urquan_native::urquan_postprocess",
        ),
        (
            crate::ships::races::samatra::engine::comet_collision as *const () as usize,
            "ships::races::samatra::comet_collision",
        ),
        (
            crate::ships::races::samatra::engine::comet_preprocess as *const () as usize,
            "ships::races::samatra::comet_preprocess",
        ),
        (
            crate::ships::races::samatra::engine::gate_collision as *const () as usize,
            "ships::races::samatra::gate_collision",
        ),
        (
            crate::ships::races::samatra::engine::gate_preprocess as *const () as usize,
            "ships::races::samatra::gate_preprocess",
        ),
        (
            crate::ships::races::samatra::engine::generator_collision as *const () as usize,
            "ships::races::samatra::generator_collision",
        ),
        (
            crate::ships::races::samatra::engine::generator_death as *const () as usize,
            "ships::races::samatra::generator_death",
        ),
        (
            crate::ships::races::samatra::engine::generator_preprocess as *const () as usize,
            "ships::races::samatra::generator_preprocess",
        ),
    ]
}
#[cfg(not(test))]
fn part4() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::samatra::engine::samatra_intelligence as *const () as usize,
            "ships::races::samatra::samatra_intelligence",
        ),
        (
            crate::ships::races::samatra::engine::samatra_postprocess as *const () as usize,
            "ships::races::samatra::samatra_postprocess",
        ),
        (
            crate::ships::races::samatra::engine::samatra_preprocess as *const () as usize,
            "ships::races::samatra::samatra_preprocess",
        ),
        (
            crate::ships::races::samatra::engine::sentinel_collision as *const () as usize,
            "ships::races::samatra::sentinel_collision",
        ),
        (
            crate::ships::races::samatra::engine::sentinel_preprocess as *const () as usize,
            "ships::races::samatra::sentinel_preprocess",
        ),
        (
            crate::ships::races::samatra::engine::turret_preprocess as *const () as usize,
            "ships::races::samatra::turret_preprocess",
        ),
        (
            crate::ships::races::yehat_native::engine::yehat_intelligence as *const () as usize,
            "ships::races::yehat_native::yehat_intelligence",
        ),
        (
            crate::ships::races::yehat_native::engine::yehat_postprocess as *const () as usize,
            "ships::races::yehat_native::yehat_postprocess",
        ),
        (
            crate::ships::races::yehat_native::engine::yehat_preprocess as *const () as usize,
            "ships::races::yehat_native::yehat_preprocess",
        ),
        (
            crate::ships::races::utwig_native::engine::initialize_lance as *const () as usize,
            "ships::races::utwig_native::initialize_lance",
        ),
    ]
}
#[cfg(not(test))]
fn part5() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::utwig_native::engine::utwig_collision as *const () as usize,
            "ships::races::utwig_native::utwig_collision",
        ),
        (
            crate::ships::races::utwig_native::engine::utwig_intelligence as *const () as usize,
            "ships::races::utwig_native::utwig_intelligence",
        ),
        (
            crate::ships::races::utwig_native::engine::utwig_preprocess as *const () as usize,
            "ships::races::utwig_native::utwig_preprocess",
        ),
        (
            crate::ships::races::chenjesu_native::engine::chenjesu_preprocess as *const () as usize,
            "ships::races::chenjesu_native::chenjesu_preprocess",
        ),
        (
            crate::ships::races::chenjesu_native::engine::crystal_collision as *const () as usize,
            "ships::races::chenjesu_native::crystal_collision",
        ),
        (
            crate::ships::races::chenjesu_native::engine::crystal_postprocess as *const () as usize,
            "ships::races::chenjesu_native::crystal_postprocess",
        ),
        (
            crate::ships::races::chenjesu_native::engine::crystal_preprocess as *const () as usize,
            "ships::races::chenjesu_native::crystal_preprocess",
        ),
        (
            crate::ships::races::chenjesu_native::engine::doggy_collision as *const () as usize,
            "ships::races::chenjesu_native::doggy_collision",
        ),
        (
            crate::ships::races::chenjesu_native::engine::doggy_death as *const () as usize,
            "ships::races::chenjesu_native::doggy_death",
        ),
    ]
}
#[cfg(not(test))]
fn part6() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::chenjesu_native::engine::doggy_preprocess as *const () as usize,
            "ships::races::chenjesu_native::doggy_preprocess",
        ),
        (
            crate::ships::races::chenjesu_native::engine::initialize_crystal as *const () as usize,
            "ships::races::chenjesu_native::initialize_crystal",
        ),
        (
            crate::ships::races::chenjesu_native::engine::spawn_doggy as *const () as usize,
            "ships::races::chenjesu_native::spawn_doggy",
        ),
        (
            crate::ships::races::mmrnmhrm_native::engine::missile_preprocess as *const () as usize,
            "ships::races::mmrnmhrm_native::missile_preprocess",
        ),
        (
            crate::ships::races::mmrnmhrm_native::engine::mmrnmhrm_preprocess as *const () as usize,
            "ships::races::mmrnmhrm_native::mmrnmhrm_preprocess",
        ),
        (
            crate::ships::races::melnorme_native::engine::confuse_preprocess as *const () as usize,
            "ships::races::melnorme_native::confuse_preprocess",
        ),
        (
            crate::ships::races::melnorme_native::engine::confusion_collision as *const () as usize,
            "ships::races::melnorme_native::confusion_collision",
        ),
        (
            crate::ships::races::melnorme_native::engine::initialize_pump_up as *const () as usize,
            "ships::races::melnorme_native::initialize_pump_up",
        ),
        (
            crate::ships::races::melnorme_native::engine::pump_up_collision as *const () as usize,
            "ships::races::melnorme_native::pump_up_collision",
        ),
    ]
}
#[cfg(not(test))]
fn part7() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::melnorme_native::engine::pump_up_postprocess as *const () as usize,
            "ships::races::melnorme_native::pump_up_postprocess",
        ),
        (
            crate::ships::races::melnorme_native::engine::pump_up_preprocess as *const () as usize,
            "ships::races::melnorme_native::pump_up_preprocess",
        ),
        (
            crate::ships::races::shofixti_native::engine::destruct_preprocess as *const () as usize,
            "ships::races::shofixti_native::destruct_preprocess",
        ),
        (
            crate::ships::races::shofixti_native::engine::self_destruct as *const () as usize,
            "ships::races::shofixti_native::self_destruct",
        ),
        (
            crate::ships::races::chmmr_native::engine::chmmr_intelligence as *const () as usize,
            "ships::races::chmmr_native::chmmr_intelligence",
        ),
        (
            crate::ships::races::chmmr_native::engine::chmmr_postprocess as *const () as usize,
            "ships::races::chmmr_native::chmmr_postprocess",
        ),
        (
            crate::ships::races::chmmr_native::engine::chmmr_preprocess as *const () as usize,
            "ships::races::chmmr_native::chmmr_preprocess",
        ),
        (
            crate::ships::races::chmmr_native::engine::laser_death as *const () as usize,
            "ships::races::chmmr_native::laser_death",
        ),
        (
            crate::ships::races::chmmr_native::engine::satellite_collision as *const () as usize,
            "ships::races::chmmr_native::satellite_collision",
        ),
    ]
}
#[cfg(not(test))]
fn part8() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::chmmr_native::engine::satellite_death as *const () as usize,
            "ships::races::chmmr_native::satellite_death",
        ),
        (
            crate::ships::races::chmmr_native::engine::satellite_postprocess as *const () as usize,
            "ships::races::chmmr_native::satellite_postprocess",
        ),
        (
            crate::ships::races::chmmr_native::engine::satellite_preprocess as *const () as usize,
            "ships::races::chmmr_native::satellite_preprocess",
        ),
        (
            crate::ships::races::chmmr_native::engine::spawn_point_defense as *const () as usize,
            "ships::races::chmmr_native::spawn_point_defense",
        ),
        (
            crate::ships::races::chmmr_native::engine::spawn_satellites as *const () as usize,
            "ships::races::chmmr_native::spawn_satellites",
        ),
        (
            crate::ships::races::androsynth_native::engine::blazer_collision as *const () as usize,
            "ships::races::androsynth_native::blazer_collision",
        ),
        (
            crate::ships::races::androsynth_native::engine::bubble_preprocess as *const () as usize,
            "ships::races::androsynth_native::bubble_preprocess",
        ),
        (
            crate::ships::races::androsynth_native::engine::initialize_bubble as *const () as usize,
            "ships::races::androsynth_native::initialize_bubble",
        ),
        (
            crate::ships::races::ilwrath_native::engine::flame_collision as *const () as usize,
            "ships::races::ilwrath_native::flame_collision",
        ),
        (
            crate::ships::races::ilwrath_native::engine::flame_preprocess as *const () as usize,
            "ships::races::ilwrath_native::flame_preprocess",
        ),
    ]
}
#[cfg(not(test))]
fn part9() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::ilwrath_native::engine::ilwrath_intelligence as *const () as usize,
            "ships::races::ilwrath_native::ilwrath_intelligence",
        ),
        (
            crate::ships::races::ilwrath_native::engine::ilwrath_preprocess as *const () as usize,
            "ships::races::ilwrath_native::ilwrath_preprocess",
        ),
        (
            crate::ships::races::ilwrath_native::engine::initialize_flame as *const () as usize,
            "ships::races::ilwrath_native::initialize_flame",
        ),
        (
            crate::ships::races::syreen_native::engine::initialize_dagger as *const () as usize,
            "ships::races::syreen_native::initialize_dagger",
        ),
        (
            crate::ships::races::syreen_native::engine::spawn_crew as *const () as usize,
            "ships::races::syreen_native::spawn_crew",
        ),
        (
            crate::ships::races::syreen_native::engine::syreen_intelligence as *const () as usize,
            "ships::races::syreen_native::syreen_intelligence",
        ),
        (
            crate::ships::races::syreen_native::engine::syreen_postprocess as *const () as usize,
            "ships::races::syreen_native::syreen_postprocess",
        ),
        (
            crate::ships::races::supox_native::engine::initialize_horn as *const () as usize,
            "ships::races::supox_native::initialize_horn",
        ),
        (
            crate::ships::races::supox_native::engine::supox_intelligence as *const () as usize,
            "ships::races::supox_native::supox_intelligence",
        ),
        (
            crate::ships::races::supox_native::engine::supox_preprocess as *const () as usize,
            "ships::races::supox_native::supox_preprocess",
        ),
    ]
}
#[cfg(not(test))]
fn part10() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::druuge_native::engine::cannon_collision as *const () as usize,
            "ships::races::druuge_native::cannon_collision",
        ),
        (
            crate::ships::races::druuge_native::engine::druuge_intelligence as *const () as usize,
            "ships::races::druuge_native::druuge_intelligence",
        ),
        (
            crate::ships::races::druuge_native::engine::druuge_postprocess as *const () as usize,
            "ships::races::druuge_native::druuge_postprocess",
        ),
        (
            crate::ships::races::druuge_native::engine::druuge_preprocess as *const () as usize,
            "ships::races::druuge_native::druuge_preprocess",
        ),
        (
            crate::ships::races::druuge_native::engine::initialize_cannon as *const () as usize,
            "ships::races::druuge_native::initialize_cannon",
        ),
        (
            crate::ships::races::mycon_native::engine::initialize_plasma as *const () as usize,
            "ships::races::mycon_native::initialize_plasma",
        ),
        (
            crate::ships::races::mycon_native::engine::mycon_intelligence as *const () as usize,
            "ships::races::mycon_native::mycon_intelligence",
        ),
        (
            crate::ships::races::mycon_native::engine::mycon_postprocess as *const () as usize,
            "ships::races::mycon_native::mycon_postprocess",
        ),
        (
            crate::ships::races::mycon_native::engine::plasma_collision as *const () as usize,
            "ships::races::mycon_native::plasma_collision",
        ),
        (
            crate::ships::races::mycon_native::engine::plasma_preprocess as *const () as usize,
            "ships::races::mycon_native::plasma_preprocess",
        ),
    ]
}
#[cfg(not(test))]
fn part11() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::arilou_native::engine::arilou_intelligence as *const () as usize,
            "ships::races::arilou_native::arilou_intelligence",
        ),
        (
            crate::ships::races::arilou_native::engine::arilou_preprocess as *const () as usize,
            "ships::races::arilou_native::arilou_preprocess",
        ),
        (
            crate::ships::races::vux_native::engine::limpet_collision as *const () as usize,
            "ships::races::vux_native::limpet_collision",
        ),
        (
            crate::ships::races::vux_native::engine::limpet_preprocess as *const () as usize,
            "ships::races::vux_native::limpet_preprocess",
        ),
        (
            crate::ships::races::vux_native::engine::spawn_limpets as *const () as usize,
            "ships::races::vux_native::spawn_limpets",
        ),
        (
            crate::ships::races::vux_native::engine::vux_intelligence as *const () as usize,
            "ships::races::vux_native::vux_intelligence",
        ),
        (
            crate::ships::races::vux_native::engine::vux_postprocess as *const () as usize,
            "ships::races::vux_native::vux_postprocess",
        ),
        (
            crate::ships::races::vux_native::engine::vux_preprocess as *const () as usize,
            "ships::races::vux_native::vux_preprocess",
        ),
        (
            crate::ships::races::zoqfot_native::engine::initialize_spit as *const () as usize,
            "ships::races::zoqfot_native::initialize_spit",
        ),
        (
            crate::ships::races::zoqfot_native::engine::spawn_tongue as *const () as usize,
            "ships::races::zoqfot_native::spawn_tongue",
        ),
    ]
}
#[cfg(not(test))]
fn part12() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::zoqfot_native::engine::spit_preprocess as *const () as usize,
            "ships::races::zoqfot_native::spit_preprocess",
        ),
        (
            crate::ships::races::zoqfot_native::engine::tongue_collision as *const () as usize,
            "ships::races::zoqfot_native::tongue_collision",
        ),
        (
            crate::ships::races::zoqfot_native::engine::tongue_postprocess as *const () as usize,
            "ships::races::zoqfot_native::tongue_postprocess",
        ),
        (
            crate::ships::races::zoqfot_native::engine::zoqfotpik_postprocess as *const () as usize,
            "ships::races::zoqfot_native::zoqfotpik_postprocess",
        ),
        (
            crate::ships::races::umgah_native::engine::cone_collision as *const () as usize,
            "ships::races::umgah_native::cone_collision",
        ),
        (
            crate::ships::races::umgah_native::engine::cone_preprocess as *const () as usize,
            "ships::races::umgah_native::cone_preprocess",
        ),
        (
            crate::ships::races::umgah_native::engine::initialize_cone as *const () as usize,
            "ships::races::umgah_native::initialize_cone",
        ),
        (
            crate::ships::races::umgah_native::engine::umgah_intelligence as *const () as usize,
            "ships::races::umgah_native::umgah_intelligence",
        ),
        (
            crate::ships::races::umgah_native::engine::umgah_postprocess as *const () as usize,
            "ships::races::umgah_native::umgah_postprocess",
        ),
        (
            crate::ships::races::umgah_native::engine::umgah_preprocess as *const () as usize,
            "ships::races::umgah_native::umgah_preprocess",
        ),
    ]
}
#[cfg(not(test))]
fn part13() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::thraddash_native::engine::initialize_horn as *const () as usize,
            "ships::races::thraddash_native::initialize_horn",
        ),
        (
            crate::ships::races::sis_native::engine::blaster_collision as *const () as usize,
            "ships::races::sis_native::blaster_collision",
        ),
        (
            crate::ships::races::sis_native::engine::blaster_preprocess as *const () as usize,
            "ships::races::sis_native::blaster_preprocess",
        ),
        (
            crate::ships::races::sis_native::engine::initialize_blasters as *const () as usize,
            "ships::races::sis_native::initialize_blasters",
        ),
        (
            crate::ships::races::sis_native::engine::sis_battle_postprocess as *const () as usize,
            "ships::races::sis_native::sis_battle_postprocess",
        ),
        (
            crate::ships::races::sis_native::engine::sis_battle_preprocess as *const () as usize,
            "ships::races::sis_native::sis_battle_preprocess",
        ),
        (
            crate::ships::races::sis_native::engine::sis_hyper_postprocess as *const () as usize,
            "ships::races::sis_native::sis_hyper_postprocess",
        ),
        (
            crate::ships::races::sis_native::engine::sis_hyper_preprocess as *const () as usize,
            "ships::races::sis_native::sis_hyper_preprocess",
        ),
        (
            crate::ships::races::sis_native::engine::sis_intelligence as *const () as usize,
            "ships::races::sis_native::sis_intelligence",
        ),
        (
            crate::ships::races::sis_native::engine::spawn_point_defense as *const () as usize,
            "ships::races::sis_native::spawn_point_defense",
        ),
    ]
}
#[cfg(not(test))]
fn part14() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::blackurq_native::engine::buzzsaw_collision as *const () as usize,
            "ships::races::blackurq_native::buzzsaw_collision",
        ),
        (
            crate::ships::races::blackurq_native::engine::buzzsaw_postprocess as *const () as usize,
            "ships::races::blackurq_native::buzzsaw_postprocess",
        ),
        (
            crate::ships::races::blackurq_native::engine::buzzsaw_preprocess as *const () as usize,
            "ships::races::blackurq_native::buzzsaw_preprocess",
        ),
        (
            crate::ships::races::blackurq_native::engine::gas_cloud_collision as *const () as usize,
            "ships::races::blackurq_native::gas_cloud_collision",
        ),
        (
            crate::ships::races::blackurq_native::engine::initialize_buzzsaw as *const () as usize,
            "ships::races::blackurq_native::initialize_buzzsaw",
        ),
        (
            crate::ships::races::blackurq_native::engine::spawn_gas_cloud as *const () as usize,
            "ships::races::blackurq_native::spawn_gas_cloud",
        ),
        (
            crate::ships::races::blackurq_native::engine::spin_preprocess as *const () as usize,
            "ships::races::blackurq_native::spin_preprocess",
        ),
        (
            crate::ships::races::blackurq_native::engine::splinter_preprocess as *const () as usize,
            "ships::races::blackurq_native::splinter_preprocess",
        ),
    ]
}
