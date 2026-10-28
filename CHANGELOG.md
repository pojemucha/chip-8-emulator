# Changelog

All notable changes to this project will be documented in this file. 
The documentation is based on [Keep a changelog](https://keepachangelog.com/en/1.1.0/)

## [Unreleased]

## [0.4.0]

### Added
- Fetch-decode-execute cycle.
- Fetch: Implemented combining two 8-bit bytes into 16-bit instruction.
- Decode: Reads instructions and parses them into individual nibbles ('opcode', 'x', 'y', 'n', 'nn', 'nnn').
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
- Basic Rust repository structure ('cargo init').
- Configured a local GitLab Runner for fast incremental builds.
- Added '.gitlab-ci.yml' configuration with support for automatic Linux builds.
- Configured cross-compilation and artifact ('.exe') preservation for the Windows platform ('x86-64-pc-windows-gnu').
- 'README.md' and 'CHANGELOG.md' files
