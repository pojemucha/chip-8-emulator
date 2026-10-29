use std::io;

use rand::Rng;

#[allow(dead_code)]
const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[allow(dead_code)]
const MAX_ALLOWED_SIZE: usize = 3584;

#[allow(dead_code)]
struct Cpu {
    memory: [u8; 4096],
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: [u16; 16],
    sp: usize,

    display: [bool; 64 * 32],

    delay_timer: u8,
    sound_timer: u8,

    keypad: [bool; 16],
}

#[allow(dead_code)]
impl Cpu {
    pub fn new() -> Self {
        let mut chip8 = Self {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            stack: [0; 16],
            sp: 0,

            display: [false; 64 * 32],

            delay_timer: 0,
            sound_timer: 0,

            keypad: [false; 16],
        };

        chip8.memory[0x050..0x0A0].copy_from_slice(&FONTSET);

        chip8
    }

    pub fn load_rom(&mut self, buffer: &[u8]) -> io::Result<()> {
        let buffer_size = buffer.len();

        if buffer_size > MAX_ALLOWED_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "ROM is too large ({} byte), max: {} byte",
                    buffer_size, MAX_ALLOWED_SIZE
                ),
            ));
        }

        self.memory[0x200..0x200 + buffer_size].copy_from_slice(buffer);
        Ok(())
    }

    pub fn fetch(&self) -> u16 {
        let high_byte: u8 = self.memory[self.pc as usize];
        let low_byte: u8 = self.memory[(self.pc + 1) as usize];

        ((high_byte as u16) << 8) | (low_byte as u16)
    }

    pub fn decode(&self, instruction: u16) -> (u8, u8, u8, u8, u8, u16) {
        let second_byte_mask: u8 = 0x0F;
        let no_first_byte_mask: u16 = 0x0FFF;

        let nn: u8 = instruction as u8;
        let nnn: u16 = instruction & no_first_byte_mask;

        let opcode: u8 = ((instruction >> 12) as u8) & second_byte_mask;
        let x: u8 = ((instruction >> 8) as u8) & second_byte_mask;
        let y: u8 = ((instruction >> 4) as u8) & second_byte_mask;
        let n: u8 = nn & second_byte_mask;

        (opcode, x, y, n, nn, nnn)
    }

    pub fn execute(&mut self, opcode: u8, x: u8, y: u8, n: u8, nn: u8, nnn: u16) {
        match (opcode, x, y, n) {
            // Subroutines
            // JUMP (1NNN)
            (0x1, _, _, _) => self.pc = nnn,
            // CALL (2NNN)
            (0x2, _, _, _) => {
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            }
            // RET (00EE)
            (0x0, 0x0, 0xE, 0xE) => {
                self.sp -= 1;
                self.pc = self.stack[self.sp];
            }

            // Skip conditionally
            // 3XNN
            (0x3, _, _, _) => {
                if self.v[x as usize] == nn {
                    self.pc += 2;
                }
            }
            // 4XNN
            (0x4, _, _, _) => {
                if self.v[x as usize] != nn {
                    self.pc += 2;
                }
            }
            // 5XY0
            (0x5, _, _, 0x0) => {
                if self.v[x as usize] == self.v[y as usize] {
                    self.pc += 2;
                }
            }
            // 9XY0
            (0x9, _, _, 0x0) => {
                if self.v[x as usize] != self.v[y as usize] {
                    self.pc += 2;
                }
            }
            // SET (6XNN)
            (0x6, _, _, _) => {
                self.v[x as usize] = nn;
            }
            // ADD (7XNN)
            (0x7, _, _, _) => {
                self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
            }

            // Logic and arithmetic
            // Set (8XY0)
            (0x8, _, _, 0x0) => {
                self.v[x as usize] = self.v[y as usize];
            }
            // Binary OR (8XY1)
            (0x8, _, _, 0x1) => {
                self.v[x as usize] |= self.v[y as usize];
            }
            // Binary AND (8XY2)
            (0x8, _, _, 0x2) => {
                self.v[x as usize] &= self.v[y as usize];
            }
            // Logical XOR (8XY3)
            (0x8, _, _, 0x3) => {
                self.v[x as usize] ^= self.v[y as usize];
            }
            // Add (8XY4)
            (0x8, _, _, 0x4) => {
                (self.v[x as usize], self.v[0xF]) = {
                    let (res, overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
                    (res, overflow as u8)
                }
            }
            // Substract (8XY5) [vx - vy]
            (0x8, _, _, 0x5) => {
                (self.v[x as usize], self.v[0xF]) = {
                    let (res, overflow) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
                    (res, !overflow as u8)
                }
            }
            // Shift (8XY6) [right]
            (0x8, _, _, 0x6) => {
                let dropped_bit = self.v[x as usize] & 1;

                self.v[x as usize] >>= 1;

                if dropped_bit != 0 {
                    self.v[0xF] = 1;
                } else {
                    self.v[0xF] = 0;
                }
            }
            // Substruct (8XY7) [vy - vx]
            (0x8, _, _, 0x7) => {
                (self.v[x as usize], self.v[0xF]) = {
                    let (res, overflow) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
                    (res, !overflow as u8)
                }
            }
            // Shift (8XYE) [left]
            (0x8, _, _, 0xE) => {
                let dropped_bit: u8 = (self.v[x as usize] >> 7) & 1;

                self.v[x as usize] <<= 1;

                if dropped_bit != 0 {
                    self.v[0xF] = 1;
                } else {
                    self.v[0xF] = 0;
                }
            }
            // Set index (ANNN)
            (0xA, _, _, _) => {
                self.i = nnn;
            }
            // Jump with offset (BNNN)
            (0xB, _, _, _) => {
                self.pc = nnn + (self.v[0x0] as u16);
            }
            // Random (CXNN)
            (0xC, _, _, _) => {
                let mut rng = rand::thread_rng();
                let random_byte: u8 = rng.gen_range(0..=255);

                self.v[x as usize] = nn & random_byte;
            }

            // Drawing
            // Clear screen (00E0)
            (0x0, 0x0, 0xE, 0x0) => {
                self.display = [false; 64 * 32];
            }
            // Display (DXYN)
            (0xD, _, _, _) => {
                let x_coord: u8 = self.v[x as usize] % 64;
                let y_coord: u8 = self.v[y as usize] % 32;

                self.v[0xF] = 0x0;

                for row in 0..n {
                    let current_y = y_coord + row;

                    if current_y > 31 {
                        break;
                    }

                    let sprite_byte: u8 = self.memory[(self.i + row as u16) as usize];

                    for col in 0..8 {
                        let current_x = x_coord + col;

                        if current_x > 63 {
                            break;
                        }

                        let coord: usize = (current_y as usize) * 64 + (current_x as usize);

                        if (sprite_byte & ((1 << 7) >> col)) != 0 {
                            if self.display[coord] {
                                self.v[0xF] = 1;
                            }
                            self.display[coord] ^= true;
                        }
                    }
                }
            }

            // Timers
            // FX07
            (0xF, _, 0x0, 0x7) => {
                self.v[x as usize] = self.delay_timer;
            }
            // FX15
            (0xF, _, 0x1, 0x5) => {
                self.delay_timer = self.v[x as usize];
            }
            // FX18
            (0xF, _, 0x1, 0x8) => {
                self.sound_timer = self.v[x as usize];
            }
            // Skip if key
            // EX9E
            (0xE, _, 0x9, 0xE) => {
                if self.keypad[self.v[x as usize] as usize] {
                    self.pc += 2;
                }
            }
            // EXA1
            (0xE, _, 0xA, 0x1) => {
                if !self.keypad[self.v[x as usize] as usize] {
                    self.pc += 2;
                }
            }

            // Add to index (FX1E)
            (0xF, _, 0x1, 0xE) => {
                self.i = (self.i + self.v[x as usize] as u16) & 0x0FFF;
            }
            // Get key (FX0A)
            (0xF, _, 0x0, 0xA) => {
                for i in 0..=0xF {
                    if self.keypad[i] {
                        self.v[x as usize] = i as u8;
                        return;
                    }
                }

                self.pc -= 2;
            }
            // Font character (FX29)
            (0xF, _, 0x2, 0x9) => {
                self.i = 0x050 + (self.v[x as usize] as u16) * 5;
            }
            // Binary-coded decimal conversion (FX33)
            (0xF, _, 0x3, 0x3) => {
                let vx: u8 = self.v[x as usize];
                self.memory[self.i as usize] = vx / 100;
                self.memory[(self.i + 1) as usize] = (vx % 100) / 10;
                self.memory[(self.i + 2) as usize] = vx % 10;
            }

            // Store and load memory
            // FX55
            (0xF, _, 0x5, 0x5) => {
                for i in 0..=x {
                    self.memory[(self.i + i as u16) as usize] = self.v[i as usize];
                }

                self.i += (x + 1) as u16;
            }
            // FX65
            (0xF, _, 0x6, 0x5) => {
                for i in 0..=x {
                    self.v[i as usize] = self.memory[(self.i + i as u16) as usize];
                }

                self.i += (x + 1) as u16;
            }

            (_, _, _, _) => (),
        }
    }

    fn tick(&mut self) -> io::Result<()> {
        if self.pc > 4094 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PC out of bounds",
            ));
        }

        let instruction: u16 = self.fetch();

        self.pc += 2;

        let (opcode, x, y, n, nn, nnn) = self.decode(instruction);

        self.execute(opcode, x, y, n, nn, nnn);

        Ok(())
    }

    pub fn update_timers(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fontset_loading() {
        let chip8 = Cpu::new();

        assert_eq!(&chip8.memory[0x050..0x0A0], &FONTSET);
    }

    #[test]
    fn test_rom_loading() {
        let rom: [u8; 3] = [0x11, 0x22, 0x33];
        let mut chip8 = Cpu::new();

        chip8.load_rom(&rom).unwrap();

        assert_eq!(&chip8.memory[0x200..0x200 + 3], &rom);
    }

    #[test]
    fn test_rom_too_large() {
        let rom = [0; MAX_ALLOWED_SIZE + 1];
        let mut chip8 = Cpu::new();

        assert!(chip8.load_rom(&rom).is_err());
    }

    #[test]
    fn test_jump_innn() {
        let mut chip8 = Cpu::new();

        let (opcode, x, y, n, nn, nnn) = chip8.decode(0x1ABC);
        chip8.execute(opcode, x, y, n, nn, nnn);

        assert_eq!(chip8.pc, 0xABC);
    }

    #[test]
    fn test_skip_if_equal() {
        let mut chip8 = Cpu::new();

        let (opcode, x, y, n, nn, nnn) = chip8.decode(0x6A22);
        chip8.execute(opcode, x, y, n, nn, nnn);

        let (opcode, x, y, n, nn, nnn) = chip8.decode(0x3A22);
        chip8.execute(opcode, x, y, n, nn, nnn);

        assert_eq!(chip8.pc, 0x200 + 2);
    }

    #[test]
    fn test_tick_out_of_bounds() {
        let mut chip8 = Cpu::new();

        chip8.pc = 4095;

        assert!(chip8.tick().is_err());
    }

    #[test]
    fn test_draw_sprite() {
        let mut chip8 = Cpu::new();

        chip8.i = 0x200;
        chip8.memory[chip8.i as usize] = 0b10000000;

        let (opcode, x, y, n, nn, nnn) = chip8.decode(0xD011);
        chip8.execute(opcode, x, y, n, nn, nnn);

        let mut right_condition: [bool; 64 * 32] = [false; 64 * 32];
        right_condition[0] = true;

        assert_eq!(chip8.display, right_condition);
        assert_eq!(chip8.v[0xF], 0);

        chip8.execute(opcode, x, y, n, nn, nnn);

        right_condition[0] = false;

        assert_eq!(chip8.display, right_condition);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn test_wait_keypress() {
        let mut chip8 = Cpu::new();

        chip8.memory[0x200] = 0xF3;
        chip8.memory[0x201] = 0x0A;

        assert!(chip8.tick().is_ok());
        assert_eq!(chip8.pc, 0x200);

        chip8.keypad[0x5] = true;

        assert!(chip8.tick().is_ok());
        assert_eq!(chip8.v[3], 0x5);
        assert_eq!(chip8.pc, 0x200 + 2);
    }

    #[test]
    fn test_math_add_with_carry() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 254;
        chip8.v[0x1] = 1;

        let (opcode, x, y, n, nn, nnn) = chip8.decode(0x8014);
        chip8.execute(opcode, x, y, n, nn, nnn);

        assert_eq!(chip8.v[0x0], 255);
        assert_eq!(chip8.v[0x1], 1);
        assert_eq!(chip8.v[0xF], 0);

        chip8.execute(opcode, x, y, n, nn, nnn);

        assert_eq!(chip8.v[0x0], 0);
        assert_eq!(chip8.v[0x1], 1);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn test_decimal_conversion() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 156;
        chip8.i = 0x200;

        let (opcode, x, y, n, nn, nnn) = chip8.decode(0xF033);
        chip8.execute(opcode, x, y, n, nn, nnn);

        assert_eq!(chip8.i, 0x200);
        assert_eq!(chip8.v[x as usize], 156);
        assert_eq!(
            &chip8.memory[(chip8.i as usize)..(chip8.i + 3) as usize],
            &[1, 5, 6]
        );
    }
}
