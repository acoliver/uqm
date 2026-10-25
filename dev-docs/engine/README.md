# Engine references

These upstream documents describe content formats and original engine algorithms.
They were moved from `sc2/doc/devel` and the old UIO source directory. C filenames,
source layouts and proposed features in them are historical. Check the Rust
implementation before relying on an API or implementation detail.

## Formats and content

- [Animation](aniformat), [graphics](gfxres), [fonts](fontres) and [string tables](strtab)
- [Resources](resources), [content files](contentfiles) and [race strings](racestrings)
- [Music resources](musicres), [sound effects](sfx) and [voice effects](voiceeffects)
- [3DO packages](pkgformat) and [Star Control I formats](sc1)
- [Saves](savefile), [state files](statefiles) and [Melee teams](meleeteams)

## Engine behaviour

- [Planet generation](generate), [original generation](orggenerate),
  [topography](planettopo), [rendering](planetrender) and [rotation](planetrotate)
- [Graphics library](gfxlib) and [graphics versions](gfxversions)
- [Input](input), [battle input](battleinput), [dialogue](dialogs) and [scripts](script)
- [Queues](queues), [threads](threads), [timing](timing) and [glossary](glossary)
- [Netplay protocol](netplay/protocol), [states](netplay/states) and [notes](netplay/notes)
- [UIO basics](uio/basics) and [conventions](uio/conventions)

[Historical notes](historical), [upstream version history](versions) and the old
[data module design](plugins) are retained as reference, not development plans.
[MD5 attribution](md5-attribution.txt) records the origin of the former C MD5
implementation. Its removal does not remove that attribution.

The upstream manual is now [docs/manual.txt](../../docs/manual.txt).
The upstream man page remains at `sc2/doc/users/uqm.6`; it is historical and is
not installed by the current release packager. Current player instructions are
in [docs/](../../docs/).
