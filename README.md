# CHIP-8 Emulator

Chip-8 emulator is a popular way to start building emulators and to understand how the machine architecture works. 

In our implementation we introduce a single launcher to run your beloved `.ch8` games.

For our team this project is a way to learn Rust and to learn more about git, work in team and etc.

## Technology stack
* **Language:** Rust
* **UI Framework:** Slint
* **CI/CD:** GitHub Actions

## Build Prequiseites

To develop and build the project locally, you need the Rust toolchain installed.

1. Install Rust (stable):

   Download manually from the [official website](https://rust-lang.org/tools/install/ "Official rust installation instruction") or download it via terminal:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository and navigate to the project folder:

   ```bash
   git clone https://github.com/pojemucha/chip-8-emulator
   cd chip-8-emulator
   ```

3. Build the project:

    ```bash
    cargo build --release
    ```
    
Or just take build from Releases.

4. Run the program:
   
   ```bash
   cd target/release/
   ```
   
   for Windows users:

   ```bash
   ./Chip-8_Emulator.exe
   ```

   for Normies:

   ```bash
   ./Chip-8_Emulator
   ```


## Looking for ROMs (games)?

We were using ROMs and test units from this users and repositories:

[loktar00 (Jason)](https://github.com/loktar00): https://github.com/loktar00/chip8/tree/master/roms,

[kripod (Kristóf Poduszló)](https://github.com/kripod): https://github.com/kripod/chip8-roms/tree/master/games.

## How to use our program?

Immedeatly after launching the app you will be in the library, where you can toggle settings (change active or background colour palette, change CPU frequency) and add ROMs. 
You can simply press button 'Import ROM' and load your game via file manager, or you can be a cool guy and add it via drag-&-drop mechanic (pick up the executable file from file manager or desktop and drop it on library window).

We added a full support of 16 keys:

   1 (0x1), 2 (0x2), 3 (0x3), 4 (0xC),

   Q (0x4), W (0x5), E (0x6), R (0xD),
   
   A (0x7), S (0x8), D (0x9), F (0xE),
   
   Z (0xA), X (0x0), C (0xB), V (0xF).

So check your games layout to take control of your character.
