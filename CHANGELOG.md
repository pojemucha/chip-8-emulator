# Changelog

All notable changes to this project will be documented in this file. 
The documentation is based on [Keep a changelog](https://keepachangelog.com/en/1.1.0/)

## [Unreleased]

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
