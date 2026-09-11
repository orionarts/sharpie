# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.2] - 2026-09-11

### Fixed

- The "2 mounts up" and "Lower deck" check boxes in the gun layouts so they
    actually have an effect
- The miscellaneous weight boxes so they immediately have an effect instead of
    requiring another change to be made first
- The number of unplaced gun mounts were being correctly shown as being in Group
    1 "on deck" but were not actually being "placed", resulting in incorrect
    reports and underlying calculations

## [0.5.1] - 2026-09-11

### Fixed

- Windows build so that sharpie actually runs without crashing on launch

## [0.5.0] - 2026-09-10

### Added

- A full GUI that largely copies the SpringSharp 3b3 GUI. Many of the warning
    and error strings are not yet included either because the GUI does not print
    them or because the underlying library does not yet generate them
- A MacOS ARM build target

### Changed

- `convert` subcommand requires `--to` and/or `--report`
- Stern type "menu" items to match SpringSharp strings
- Use more accurate imperial<->metric conversions (exact if possible)
- Decimal parts that would print as all 0s are now truncated in the report
    (This is more heavy-handed than how SpringSharp does it). Other minor
    changes were made to the report in order to more closely match SpringSharp
- The code separation between "calculations" and "interface" was made cleaner to
    better support the GUI and any future UIs. This involved reorganiztion the
    codebase file structure, including splitting some large files into multiple
    files. The test modules were also reorganized to be more consistent in
    naming
- Freeboard deck lengths now default to their SpringSharp defaults instead of
    zero
- The `chkreport` script does a better job ignoring minor numerical formatting
    differences
- All "years" in a default Ship to the default "laid down" year of 1950
- Changed the default caliber of a battery from 0 to 45
- All values displayed values to internally store both the imperial and metric
    value so there is no "loss" when converting back and forth between
    measurement systems
- The default filename when saving an image to no longer end in "-hull"

### Fixed

- `Hull::free_cap()` now returns a lower freeboard if a ship has **any** guns
    mounted broadside, below the deck
- Missing parameters added to some gun calculations
- The "deep draft" calculation is calculated as an imperial value however if the
    hull's unit system were set to metric, "deep draft" was being returned as if
    it were a metric value, making the report incorrect. This is now fixed
- The two bulkhead types (additional and strengthened) were switched in the
    logic so what should have been calculated for one was actually being
    calculated for the other. They now function correctly
- The bunker weight calculation was fixed to prevent a crash that resulted from
    certain values for the "engine built" year

## [0.4.0] - 2026-08-27

### Added

- Add `--image` option to `convert` and `load` commands to generate an SVG of
    the hull

### Changed

- The weight of a barbette now matches SprinSharp's likely buggy behavior
- Added the missing "bulkhead beam too wide" warning to the output report

### Fixed

- Some constants were fixed to match the values in SpringSharp
- `Armor::ct_wgt()` adds both CT weights instead of just the forward weights
    twice
- When calculating `Hull::cwp()` a Transom stern takes precendence over >= 2
    engine shafts or Cb >= 0.75

## [0.3.1] - 2026-08-21

### Added

- Length parameter to `BowType::BulbForward`
- `YEAR_MIN`/`YEAR_MAX` constants bounding valid ship years
- `chkreport` script for diffing generated reports against SpringSharp output
- `GunDistributionType::None` gun distribution layout, now the default
- Crate metadata (`description`, `license`, `repository`, `rust-version`)

### Changed

- `d`/`cb` and `lwl`/`loa` pairs in `Hull` replaced with enums so each GUI widget
  maps 1:1 to a data state
- `BulkheadType::Additional` is now the default bulkhead type
- added `choice_enum!` macro to simplify enum construction for enums that define
    a list of user choices

### Fixed

- Crate dependency requirements (dropped unneeded `derive_builder`, pinned
  major/minor versions)

## [0.3.0] - 2026-08-20

### Added

- Metric and imperial unit support via a new `Measurement` type, now backing all
  hull dimensions, armor, batteries, torpedoes, mines, and derived calculations

### Fixed

- Report display bug where `fc_fwd` was assigned from `fc_len`
- Reports now match SpringSharp output more closely (strings, layout, always
  show Miscellaneous Weights)

## [0.2.0] - 2026-08-19

### Added

- `DeckType::BoxOverMachinery` and `DeckType::BoxOverBoth` deck types
- Substantially expanded test suite (CLI, units, armor weight sums, enum
  display/from-string conversions)

## [0.1.1] - 2026-05-10

### Fixed

- Allow saving a converted SpringSharp sship file to a non-existent Sharpie
  ship file
- Prevent errors when the ship's year is before 1860; provide a default year
  when none is given
- Provide a default block coefficient and handle neither `d` nor `cb` being set

## [0.1.0] - 2026-01-03

### Added

- CLI and GUI to generate reports from ship files and convert SpringSharp sship
  files to Sharpie format

[unreleased]: https://github.com/orionarts/sharpie/compare/v0.5.2...HEAD
[0.5.2]: https://github.com/orionarts/sharpie/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/orionarts/sharpie/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/orionarts/sharpie/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/orionarts/sharpie/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/orionarts/sharpie/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/orionarts/sharpie/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/orionarts/sharpie/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/orionarts/sharpie/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/orionarts/sharpie/releases/tag/v0.1.0
