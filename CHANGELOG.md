# Changelog

All notable changes to this project will be documented in this file. 
The documentation is based on [Keep a changelog](https://keepachangelog.com/en/1.1.0/)

## [Unreleased]

## [1.0.2] - 2026-11-06

### Added
- Added unit tests for ROM file extension validation and metadata extraction.

### Fixed
- Resolved external drag-and-drop issues by migrating from Slint's `DropArea` to raw `winit` window events (`DroppedFile`, `HoveredFile`).
- Added an X11 backend fallback (`WINIT_UNIX_BACKEND=x11`) on Linux to bypass Wayland drag-and-drop limitations in `winit 0.30`.

### Changed
- Updated Slint to version 1.18.1 with `unstable-winit-30` flag.

## [1.0.1] - 2026-11-06

### Changed
- Updated `README.md`: added build and run instructions (Windows/Linux), keyboard layout mapping, and updated repository links.
- Migrated the CI/CD pipeline to GitHub Actions.

### Removed
- Removed the old GitLab CI configuration file (`.gitlab-ci.yml`) due to the project migration.

## [1.0.0] - 2026-10-30

### Added
- Integrated a background thread for CPU execution, safely managed via `Arc<Mutex>` and `AtomicBool` to prevent UI freezing.
- Added continuous audio playback using the `rodio` crate, generating a 440Hz sine wave that correctly responds to the `sound_timer`.
- Implemented a duplicate check when importing ROMs (via drag-and-drop or file browser) to prevent adding the same game twice.
- Added full QWERTY / ЙЦУКЕН keyboard mapping for seamless input across different language layouts.

### Changed
- Moved the "Back to Library" button to the top navigation bar during gameplay, optimizing screen space for the emulator view.
- Exposed `sound_timer`, `display`, and `keypad` as public fields in the `Cpu` struct to allow safe external polling and mutation by the UI thread.

### Fixed
- Fixed a bug where clicking the emulator screen would drop keyboard input focus by adding a background `TouchArea` to `EmulatorView`.
- Fixed an issue causing `pc` to go out of bounds on subsequent game loads by implementing a complete state reset (`locked_cpu.clear()`) when returning to the library.

### Removed
- Removed the unused `src/ui/settings.rs` module, centralizing all configuration state within `LibraryConfig` and `EmulatorSettings`.

## [0.10.0]

### Added
- Implemented a `slint::Timer` with a 16ms interval (held in `App._timer`) to continuously refresh the emulator screen.
- Configured `EmulatorView` to use nearest-neighbor scaling (`image-rendering: pixelated; image-fit: contain`) for sharp visuals without bilinear blur.
- The settings modal now safely retains its previous values (sliders and color palettes) when reopened by pulling from shared state.

### Changed
- Optimized `convert_framebuffer` by extracting `make_mut_slice()` outside the pixel loop, significantly reducing reference-counting overhead.
- The framebuffer conversion now applies a baked 4x scale (yielding 256x128 resolution) and injects dynamic foreground/background colors on the fly.
- `on_save_settings` now updates both `MainWindow` properties and the `shared_settings` (`Rc<RefCell>`). The rendering timer reads these instantly, applying theme changes without a restart.

## [0.9.0]

### Added
- `SettingsDialog`: A new configuration modal featuring:
  - A CPU frequency slider (200–1200 Hz, default: 700 Hz).
  - RGB sliders with live previews for both active pixel (foreground) and background colors.
  - Save and Cancel action buttons.
- `MainWindow` and `EmulatorView` now support dynamic theming, applying the chosen background and foreground colors to the emulation screen immediately.
- Struct `EmulatorSettings` with `LibraryConfig` field.
- `#[serde(dufault)]` ensures old `library.json` migration.
- `on_save_settings` saves into JSON and applies changes instantly.

## [0.8.0]

### Added
- ALU instructions (`8XYN`) for bitwise operations, shifts, and arithmetic with precise `VF` carry/borrow flag handling.
- Memory manipulation and control flow instructions (`ANNN`, `BXNN`, `FX1E`, `FX29`).
- Random number generation instruction (`CXNN`) using the `rand` crate.
- Binary-Coded Decimal (BCD) conversion logic (`FX33`).
- Register memory load and store instructions (`FX55`, `FX65`).
- Unit tests validating arithmetic overflow behavior and BCD memory layout.

## [0.7.0]

### Added
- `DropArea` in `MainWindow.slint` that accepts `.ch8` files and highlights the window during drag operations.
- Import button in the header using `rfd::FileDialog` to browse and select `*.ch8` files.
- Extracted ROM name, file path, and size (formatted in B/KB) for display on game cards.
- Automatic saving of the game library to `~/.config/chip8-emulator/library.json` (Linux) or `%APPDATA%\chip8-emulator\library.json` (Windows) using `dirs` and `serde_json`.
- Configured `App::new()` to read the saved configuration on startup, populate the games model, and automatically discard entries for files that no longer exist.

## [0.6.0]

### Added
- `delay_timer`, `sound_timer` to the CPU struct.
- `update_timers` method to safely countdown to 0.
- `keypad`, a 16-element boolean array, to track state of hexadecimal keypad. 
- Timers instructions (`FX07`, `FX15`, `FX18`).
- Blocking `FX0A` instruction to loop until any key is pressed.
- Unit test to validate the blocking mechanism and successful key cupture for `FX0a` instruction.

## [0.5.0]

### Added
- 64x32 monochome display buffer
- Instructions `CLS` (Clear screen (00E0)) and `DRW` (Display (DXYN))

## [0.4.0]

### Added
- Fetch-decode-execute cycle.
- Fetch: Implemented combining two 8-bit bytes into 16-bit instruction.
- Decode: Reads instructions and parses them into individual nibbles (`opcode`, `x`, `y`, `n`, `nn`, `nnn`).
- Execute: Performs operations based on the decoded instruction.
- Tick: Steps through the cycles with a safe boundary checks.
- Tests: Unit tests verifying subroutine jumps, conditional skips, and out-of-bounds error handling.

## [0.3.0]

### Added
- Included a header bar featuring the application title and a settings button.
- Implemented adaptive layout behavior with view switching toggled via the in-game property.
- Created a flexible game library layout utilizing FlexboxLayout
- Designed a game card item with an icon placeholder, title, file size display, and click event handling.
- Added a placeholder screen for the active game state, complete with a button to return to the library launcher.
- Populated the [GameEntry] model with mock data and wired up callbacks for game-selected, back-to-launcher, and settings-clicked events.

### Changed
- main.rs hooked up ui module.

## [0.2.0]

### Added
- CPU core architecture including 4KB memory, registers (V0-VF, I, PC), and stack
- ROM loader function with memory boundary check to prevent stack overflow
- ROM loading, font layout and memory boundary test
- Hooked up the core module to the main
- cpu.rs temporary disabled dead_code warnings to prevent CI/CD fail

### Changed
- main.rs hooked core module

## [0.1.0]

### Added
- Basic Rust repository structure (`cargo init`).
- Configured a local GitLab Runner for fast incremental builds.
- Added `.gitlab-ci.yml` configuration with support for automatic Linux builds.
- Configured cross-compilation and artifact (`.exe`) preservation for the Windows platform (`x86-64-pc-windows-gnu`).
- `README.md` and `CHANGELOG.md` files
