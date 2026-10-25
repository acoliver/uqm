# Porting history

These documents retain migration designs, algorithm constraints and original-C
fixture comparisons. They are historical records. C providers, ABI cutovers,
source inventories and checkpoint status mentioned in them no longer describe
the current Rust-only build. Old local artifact paths are provenance records,
not instructions to recreate or retain those directories.

## Early subsystem designs

- [Audio mixer](PHASE4_MIXER_SPEC.md)
- [Canvas drawing](PHASE5_CANVAS_SPEC.md)
- [Input](PHASE6_INPUT_SPEC.md)
- [Alien communication](PHASE7_COMM_SPEC.md)
- [Draw command queue](DCQ_DELIVERABLES.md)
- [Graphics events](gfx_common_events_api.md)
- [Battle netplay checksums](netplay-summary.md)

## Planet algorithms and parity fixtures

- [Elevation conditioning and light maps](planets-elevation-checkpoint.md)
- [Craters, storms and gas bands](planets-terrain-checkpoint.md)
- [Oval rendering](planets-oval-checkpoint.md) and [cutover contract](planets-oval-cutover.md)
- [Sol generation and orbital behaviour](planets-sol-checkpoint.md)

For current commands and requirements, use [the development guide](../README.md).
