# CHIP-8 Emulator

Chip-8 emulator is a popular way to start building emulators and to understand how the machine architecture works. 

This project is a homework and for our team it's a way to learn Rust and to learn more about git, work in team and etc.

## Technoclogy stack
* **Language:** Rust
* **UI Framework:** Slint
* **CI/CD:** GitLab CI (with local runner)

## Build Prerequiseites

To develop and build the project locally, you need the Rust toolchain installed.

1. Install Rust (stable):

   Download manually from the [official website](https://rust-lang.org/tools/install/ "Official rust installation instruction") or download it via terminal:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository and navigate to the project folder:

   ```bash
   git clone https://vcs.uni-dubna.ru/KMS.24/chip-8-emulator.git
   cd chip-8-emulator
   ```

3. Build the project:

    ```bash
    cargo build --release
    ```
    
Or just take build from CI/CD artifacts.
