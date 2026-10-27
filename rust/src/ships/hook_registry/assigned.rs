//! Assigned callbacks with non-element first arguments and restored hooks.
#[cfg(not(test))]
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    [part0(), part1(), part2(), part3(), part4()]
        .into_iter()
        .flatten()
        .collect()
}
#[cfg(not(test))]
fn part0() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::orz_native::engine::initialize_turret_missile as *const ()
                as usize,
            "ships::races::orz_native::initialize_turret_missile",
        ),
        (
            crate::ships::races::spathi_native::engine::butt_missile_preprocess as *const ()
                as usize,
            "ships::races::spathi_native::butt_missile_preprocess",
        ),
        (
            crate::ships::races::spathi_native::engine::initialize_standard_missile as *const ()
                as usize,
            "ships::races::spathi_native::initialize_standard_missile",
        ),
        (
            crate::ships::races::slylandro_native::engine::initialize_lightning as *const ()
                as usize,
            "ships::races::slylandro_native::initialize_lightning",
        ),
        (
            crate::ships::races::slylandro_native::engine::lightning_collision as *const ()
                as usize,
            "ships::races::slylandro_native::lightning_collision",
        ),
        (
            crate::ships::races::slylandro_native::engine::lightning_postprocess as *const ()
                as usize,
            "ships::races::slylandro_native::lightning_postprocess",
        ),
        (
            crate::ships::races::slylandro_native::engine::slylandro_intelligence as *const ()
                as usize,
            "ships::races::slylandro_native::slylandro_intelligence",
        ),
        (
            crate::ships::races::slylandro_native::engine::slylandro_postprocess as *const ()
                as usize,
            "ships::races::slylandro_native::slylandro_postprocess",
        ),
        (
            crate::ships::races::slylandro_native::engine::slylandro_preprocess as *const ()
                as usize,
            "ships::races::slylandro_native::slylandro_preprocess",
        ),
        (
            crate::ships::races::pkunk_native::engine::pkunk_reincarnation_death as *const ()
                as usize,
            "ships::races::pkunk_native::pkunk_reincarnation_death",
        ),
    ]
}
#[cfg(not(test))]
fn part1() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::pkunk_native::engine::uninit_pkunk as *const () as usize,
            "ships::races::pkunk_native::uninit_pkunk",
        ),
        (
            crate::ships::races::yehat_native::engine::initialize_standard_missiles as *const ()
                as usize,
            "ships::races::yehat_native::initialize_standard_missiles",
        ),
        (
            crate::ships::races::chenjesu_native::engine::chenjesu_intelligence as *const ()
                as usize,
            "ships::races::chenjesu_native::chenjesu_intelligence",
        ),
        (
            crate::ships::races::chenjesu_native::engine::chenjesu_postprocess as *const ()
                as usize,
            "ships::races::chenjesu_native::chenjesu_postprocess",
        ),
        (
            crate::ships::races::mmrnmhrm_native::engine::initialize_dual_weapons as *const ()
                as usize,
            "ships::races::mmrnmhrm_native::initialize_dual_weapons",
        ),
        (
            crate::ships::races::mmrnmhrm_native::engine::mmrnmhrm_intelligence as *const ()
                as usize,
            "ships::races::mmrnmhrm_native::mmrnmhrm_intelligence",
        ),
        (
            crate::ships::races::mmrnmhrm_native::engine::mmrnmhrm_postprocess as *const ()
                as usize,
            "ships::races::mmrnmhrm_native::mmrnmhrm_postprocess",
        ),
        (
            crate::ships::races::mmrnmhrm_native::engine::twin_laser_collision as *const ()
                as usize,
            "ships::races::mmrnmhrm_native::twin_laser_collision",
        ),
        (
            crate::ships::races::mmrnmhrm_native::engine::uninit_mmrnmhrm as *const () as usize,
            "ships::races::mmrnmhrm_native::uninit_mmrnmhrm",
        ),
        (
            crate::ships::races::melnorme_native::engine::initialize_confusion as *const ()
                as usize,
            "ships::races::melnorme_native::initialize_confusion",
        ),
    ]
}
#[cfg(not(test))]
fn part2() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::melnorme_native::engine::initialize_test_pump_up as *const ()
                as usize,
            "ships::races::melnorme_native::initialize_test_pump_up",
        ),
        (
            crate::ships::races::melnorme_native::engine::melnorme_intelligence as *const ()
                as usize,
            "ships::races::melnorme_native::melnorme_intelligence",
        ),
        (
            crate::ships::races::melnorme_native::engine::melnorme_postprocess as *const ()
                as usize,
            "ships::races::melnorme_native::melnorme_postprocess",
        ),
        (
            crate::ships::races::shofixti_native::engine::initialize_standard_missile as *const ()
                as usize,
            "ships::races::shofixti_native::initialize_standard_missile",
        ),
        (
            crate::ships::races::shofixti_native::engine::self_destruct_kill_objects as *const ()
                as usize,
            "ships::races::shofixti_native::self_destruct_kill_objects",
        ),
        (
            crate::ships::races::shofixti_native::engine::shofixti_destruct_death as *const ()
                as usize,
            "ships::races::shofixti_native::shofixti_destruct_death",
        ),
        (
            crate::ships::races::shofixti_native::engine::shofixti_intelligence as *const ()
                as usize,
            "ships::races::shofixti_native::shofixti_intelligence",
        ),
        (
            crate::ships::races::shofixti_native::engine::shofixti_postprocess as *const ()
                as usize,
            "ships::races::shofixti_native::shofixti_postprocess",
        ),
        (
            crate::ships::races::chmmr_native::engine::initialize_megawatt_laser as *const ()
                as usize,
            "ships::races::chmmr_native::initialize_megawatt_laser",
        ),
        (
            crate::ships::races::androsynth_native::engine::androsynth_intelligence as *const ()
                as usize,
            "ships::races::androsynth_native::androsynth_intelligence",
        ),
    ]
}
#[cfg(not(test))]
fn part3() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::androsynth_native::engine::androsynth_postprocess as *const ()
                as usize,
            "ships::races::androsynth_native::androsynth_postprocess",
        ),
        (
            crate::ships::races::androsynth_native::engine::androsynth_preprocess as *const ()
                as usize,
            "ships::races::androsynth_native::androsynth_preprocess",
        ),
        (
            crate::ships::races::androsynth_native::engine::uninit_androsynth as *const () as usize,
            "ships::races::androsynth_native::uninit_androsynth",
        ),
        (
            crate::ships::races::mycon_native::engine::plasma_blast_preprocess as *const ()
                as usize,
            "ships::races::mycon_native::plasma_blast_preprocess",
        ),
        (
            crate::ships::races::arilou_native::engine::initialize_autoaim_laser as *const ()
                as usize,
            "ships::races::arilou_native::initialize_autoaim_laser",
        ),
        (
            crate::ships::races::vux_native::engine::initialize_horrific_laser as *const ()
                as usize,
            "ships::races::vux_native::initialize_horrific_laser",
        ),
        (
            crate::ships::races::zoqfot_native::engine::zoqfotpik_intelligence as *const ()
                as usize,
            "ships::races::zoqfot_native::zoqfotpik_intelligence",
        ),
        (
            crate::ships::races::umgah_native::engine::uninit_umgah as *const () as usize,
            "ships::races::umgah_native::uninit_umgah",
        ),
        (
            crate::ships::races::thraddash_native::engine::flame_napalm_preprocess as *const ()
                as usize,
            "ships::races::thraddash_native::flame_napalm_preprocess",
        ),
        (
            crate::ships::races::thraddash_native::engine::thraddash_intelligence as *const ()
                as usize,
            "ships::races::thraddash_native::thraddash_intelligence",
        ),
    ]
}
#[cfg(not(test))]
fn part4() -> Vec<(usize, &'static str)> {
    vec![
        (
            crate::ships::races::thraddash_native::engine::thraddash_preprocess as *const ()
                as usize,
            "ships::races::thraddash_native::thraddash_preprocess",
        ),
        (
            crate::ships::races::sis_native::engine::uninit_sis as *const () as usize,
            "ships::races::sis_native::uninit_sis",
        ),
        (
            crate::ships::races::blackurq_native::engine::black_urquan_intelligence as *const ()
                as usize,
            "ships::races::blackurq_native::black_urquan_intelligence",
        ),
        (
            crate::ships::races::blackurq_native::engine::black_urquan_postprocess as *const ()
                as usize,
            "ships::races::blackurq_native::black_urquan_postprocess",
        ),
        (
            crate::ships::races::blackurq_native::engine::black_urquan_preprocess as *const ()
                as usize,
            "ships::races::blackurq_native::black_urquan_preprocess",
        ),
        (
            crate::ships::races::blackurq_native::engine::buzztrack_preprocess as *const ()
                as usize,
            "ships::races::blackurq_native::buzztrack_preprocess",
        ),
        (
            crate::ships::races::blackurq_native::engine::decelerate_preprocess as *const ()
                as usize,
            "ships::races::blackurq_native::decelerate_preprocess",
        ),
        (
            crate::ships::races::blackurq_native::engine::gas_cloud_preprocess as *const ()
                as usize,
            "ships::races::blackurq_native::gas_cloud_preprocess",
        ),
    ]
}
