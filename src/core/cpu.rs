use std::io;

use rand::Rng;

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

const MAX_ALLOWED_SIZE: usize = 3584;

pub struct Cpu {
    memory: [u8; 4096],
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: [u16; 16],
    sp: usize,

    pub display: [bool; 64 * 32],

    delay_timer: u8,
    pub sound_timer: u8,

    pub keypad: [bool; 16],
}

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

    pub fn clear(&mut self) {
        *self = Self::new();
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

    pub fn decode(instruction: u16) -> (u8, u8, u8, u8, u8, u16) {
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

    pub fn execute(&mut self, opcode: u8, x: u8, y: u8, n: u8, nn: u8, nnn: u16) -> io::Result<()> {
        match (opcode, x, y, n) {
            // Subroutines
            // JUMP (1NNN)
            (0x1, _, _, _) => {
                self.pc = nnn;

                Ok(())
            }
            // CALL (2NNN)
            (0x2, _, _, _) => {
                if self.sp >= 16 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Stack overflow (max depth 16 reached)",
                    ));
                }

                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = nnn;

                Ok(())
            }
            // RET (00EE)
            (0x0, 0x0, 0xE, 0xE) => {
                if self.sp == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Stack underflow (RET called with empty stack)",
                    ));
                }

                self.sp -= 1;
                self.pc = self.stack[self.sp];

                Ok(())
            }

            // Skip conditionally
            // 3XNN
            (0x3, _, _, _) => {
                if self.v[x as usize] == nn {
                    self.pc += 2;
                }

                Ok(())
            }
            // 4XNN
            (0x4, _, _, _) => {
                if self.v[x as usize] != nn {
                    self.pc += 2;
                }

                Ok(())
            }
            // 5XY0
            (0x5, _, _, 0x0) => {
                if self.v[x as usize] == self.v[y as usize] {
                    self.pc += 2;
                }

                Ok(())
            }
            // 9XY0
            (0x9, _, _, 0x0) => {
                if self.v[x as usize] != self.v[y as usize] {
                    self.pc += 2;
                }

                Ok(())
            }
            // SET (6XNN)
            (0x6, _, _, _) => {
                self.v[x as usize] = nn;

                Ok(())
            }
            // ADD (7XNN)
            (0x7, _, _, _) => {
                self.v[x as usize] = self.v[x as usize].wrapping_add(nn);

                Ok(())
            }

            // Logic and arithmetic
            // Set (8XY0)
            (0x8, _, _, 0x0) => {
                self.v[x as usize] = self.v[y as usize];

                Ok(())
            }
            // Binary OR (8XY1)
            (0x8, _, _, 0x1) => {
                self.v[x as usize] |= self.v[y as usize];
                self.v[0xF] = 0;

                Ok(())
            }
            // Binary AND (8XY2)
            (0x8, _, _, 0x2) => {
                self.v[x as usize] &= self.v[y as usize];
                self.v[0xF] = 0;

                Ok(())
            }
            // Logical XOR (8XY3)
            (0x8, _, _, 0x3) => {
                self.v[x as usize] ^= self.v[y as usize];
                self.v[0xF] = 0;

                Ok(())
            }
            // Add (8XY4)
            (0x8, _, _, 0x4) => {
                (self.v[x as usize], self.v[0xF]) = {
                    let (res, overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
                    (res, overflow as u8)
                };

                Ok(())
            }
            // Subtract (8XY5) [vx - vy]
            (0x8, _, _, 0x5) => {
                (self.v[x as usize], self.v[0xF]) = {
                    let (res, overflow) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
                    (res, !overflow as u8)
                };

                Ok(())
            }
            // Shift (8XY6) [right]
            (0x8, _, _, 0x6) => {
                self.v[x as usize] = self.v[y as usize];

                let dropped_bit = self.v[x as usize] & 1;

                self.v[x as usize] >>= 1;

                self.v[0xF] = dropped_bit;

                Ok(())
            }
            // Subtract (8XY7) [vy - vx]
            (0x8, _, _, 0x7) => {
                (self.v[x as usize], self.v[0xF]) = {
                    let (res, overflow) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
                    (res, !overflow as u8)
                };

                Ok(())
            }
            // Shift (8XYE) [left]
            (0x8, _, _, 0xE) => {
                self.v[x as usize] = self.v[y as usize];

                let dropped_bit: u8 = (self.v[x as usize] >> 7) & 1;

                self.v[x as usize] <<= 1;

                self.v[0xF] = dropped_bit;

                Ok(())
            }
            // Set index (ANNN)
            (0xA, _, _, _) => {
                self.i = nnn;

                Ok(())
            }
            // Jump with offset (BNNN)
            (0xB, _, _, _) => {
                self.pc = nnn + (self.v[0x0] as u16);

                Ok(())
            }
            // Random (CXNN)
            (0xC, _, _, _) => {
                let mut rng = rand::thread_rng();
                let random_byte: u8 = rng.gen_range(0..=255);

                self.v[x as usize] = nn & random_byte;

                Ok(())
            }

            // Drawing
            // Clear screen (00E0)
            (0x0, 0x0, 0xE, 0x0) => {
                self.display = [false; 64 * 32];

                Ok(())
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

                    let sprite_byte: &u8 = self
                        .memory
                        .get((self.i + row as u16) as usize)
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("Memory access out of bounds at I={:#X}", self.i),
                            )
                        })?;

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

                Ok(())
            }

            // Timers
            // FX07
            (0xF, _, 0x0, 0x7) => {
                self.v[x as usize] = self.delay_timer;

                Ok(())
            }
            // FX15
            (0xF, _, 0x1, 0x5) => {
                self.delay_timer = self.v[x as usize];

                Ok(())
            }
            // FX18
            (0xF, _, 0x1, 0x8) => {
                self.sound_timer = self.v[x as usize];

                Ok(())
            }
            // Skip if key
            // EX9E
            (0xE, _, 0x9, 0xE) => {
                let key: usize = self.v[x as usize] as usize;

                let is_pressed = self.keypad.get(key).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid key index: {:#X}", key),
                    )
                })?;

                if *is_pressed {
                    self.pc += 2
                }

                Ok(())
            }
            // EXA1
            (0xE, _, 0xA, 0x1) => {
                let key: usize = self.v[x as usize] as usize;

                let is_pressed = self.keypad.get(key).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid key index: {:#X}", key),
                    )
                })?;

                if !*is_pressed {
                    self.pc += 2
                }

                Ok(())
            }

            // Add to index (FX1E)
            (0xF, _, 0x1, 0xE) => {
                self.i = (self.i + self.v[x as usize] as u16) & 0x0FFF;

                Ok(())
            }
            // Get key (FX0A)
            (0xF, _, 0x0, 0xA) => {
                for i in 0..=0xF {
                    if self.keypad[i] {
                        self.v[x as usize] = i as u8;

                        return Ok(());
                    }
                }

                self.pc -= 2;

                Ok(())
            }
            // Font character (FX29)
            (0xF, _, 0x2, 0x9) => {
                let register: u16 = self.v[x as usize] as u16;

                if register > 0xF {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid font character: {:#X}", self.v[x as usize]),
                    ));
                }

                self.i = 0x050 + register * 5;

                Ok(())
            }
            // Binary-coded decimal conversion (FX33)
            (0xF, _, 0x3, 0x3) => {
                if self.i + 2 >= 4096 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Memory access out of bounds at I={:#X}", self.i),
                    ));
                }

                let vx: u8 = self.v[x as usize];

                self.memory[self.i as usize] = vx / 100;
                self.memory[(self.i + 1) as usize] = (vx % 100) / 10;
                self.memory[(self.i + 2) as usize] = vx % 10;

                Ok(())
            }

            // Store and load memory
            // FX55
            (0xF, _, 0x5, 0x5) => {
                if self.i + (x as u16) >= 4096 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Memory access out of bounds at I={:#X}", self.i),
                    ));
                }

                for i in 0..=x {
                    self.memory[(self.i + i as u16) as usize] = self.v[i as usize];
                }

                self.i += (x + 1) as u16;

                Ok(())
            }
            // FX65
            (0xF, _, 0x6, 0x5) => {
                if self.i + (x as u16) >= 4096 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Memory access out of bounds at I={:#X}", self.i),
                    ));
                }

                for i in 0..=x {
                    self.v[i as usize] = self.memory[(self.i + i as u16) as usize];
                }

                self.i += (x + 1) as u16;

                Ok(())
            }

            (_, _, _, _) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown opcode: {:X}{:X}{:X}{:X}", opcode, x, y, n),
            )),
        }
    }

    pub fn tick(&mut self) -> io::Result<()> {
        if self.pc > 4094 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PC out of bounds",
            ));
        }

        let instruction: u16 = self.fetch();

        self.pc += 2;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(instruction);

        self.execute(opcode, x, y, n, nn, nnn)?;

        Ok(())
    }

    pub fn update_timers(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
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
    fn test_jump_1nnn() {
        let mut chip8 = Cpu::new();

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x1ABC);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.pc, 0xABC);
    }

    #[test]
    fn test_skip_if_equal_3xnn() {
        let mut chip8 = Cpu::new();

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x6A22);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x3A22);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.pc, 0x200 + 2);
    }

    #[test]
    fn test_tick_out_of_bounds() {
        let mut chip8 = Cpu::new();

        chip8.pc = 4095;

        assert!(chip8.tick().is_err());
    }

    #[test]
    fn test_draw_sprite_dxyn() {
        let mut chip8 = Cpu::new();

        chip8.i = 0x200;
        chip8.memory[chip8.i as usize] = 0b10000000;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0xD011);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        let mut right_condition: [bool; 64 * 32] = [false; 64 * 32];
        right_condition[0] = true;

        assert_eq!(chip8.display, right_condition);
        assert_eq!(chip8.v[0xF], 0);

        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        right_condition[0] = false;

        assert_eq!(chip8.display, right_condition);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn test_wait_keypress() {
        let mut chip8 = Cpu::new();

        chip8.memory[0x200] = 0xF3;
        chip8.memory[0x201] = 0x0A;

        chip8.tick().unwrap();
        assert_eq!(chip8.pc, 0x200);

        chip8.keypad[0x5] = true;

        chip8.tick().unwrap();
        assert_eq!(chip8.v[3], 0x5);
        assert_eq!(chip8.pc, 0x200 + 2);
    }

    #[test]
    fn test_math_add_with_carry_8xy4() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 254;
        chip8.v[0x1] = 1;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8014);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 255);
        assert_eq!(chip8.v[0x1], 1);
        assert_eq!(chip8.v[0xF], 0);

        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0);
        assert_eq!(chip8.v[0x1], 1);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn test_decimal_conversion_fx33() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 156;
        chip8.i = 0x200;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0xF033);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.i, 0x200);
        assert_eq!(chip8.v[x as usize], 156);
        assert_eq!(
            &chip8.memory[(chip8.i as usize)..(chip8.i + 3) as usize],
            &[1, 5, 6]
        );
    }

    #[test]
    fn test_save_overflow_flag_over_sum_8xy4() {
        let mut chip8 = Cpu::new();

        chip8.v[0xF] = 10;
        chip8.v[0x1] = 20;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8F14);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_store_memory_fx55() {
        let mut chip8 = Cpu::new();

        chip8.i = 0x300;
        chip8.v[0] = 0xAA;
        chip8.v[1] = 0xBB;
        chip8.v[2] = 0xCC;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0xF255);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(&chip8.memory[0x300..=0x302], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(chip8.i, 0x300 + 3);

        chip8.i = 4095;
        assert!(chip8.execute(opcode, x, y, n, nn, nnn).is_err());
    }

    #[test]
    fn test_load_memory_fx65() {
        let mut chip8 = Cpu::new();

        chip8.i = 0x300;
        chip8.memory[0x300] = 0xAA;
        chip8.memory[0x301] = 0xBB;
        chip8.memory[0x302] = 0xCC;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0xF265);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(&chip8.v[0..=2], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(chip8.i, 0x300 + 3);

        chip8.i = 4095;
        assert!(chip8.execute(opcode, x, y, n, nn, nnn).is_err());
    }

    #[test]
    fn test_logical_shifts_8xy6_8xye() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 0xF0;
        chip8.v[0x1] = 0xFF;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8016);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0x7F);
        assert_eq!(chip8.v[0x1], 0xFF);
        assert_eq!(chip8.v[0xF], 1);

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x801E);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0xFE);
        assert_eq!(chip8.v[0x1], 0xFF);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn test_binary_or_8xy1() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 0xF0;
        chip8.v[0x1] = 0x0F;
        chip8.v[0xF] = 1;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8011);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0xFF);
        assert_eq!(chip8.v[0x1], 0x0F);
        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_binary_and_8xy2() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 0xF0;
        chip8.v[0x1] = 0x0F;
        chip8.v[0xF] = 1;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8012);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0x00);
        assert_eq!(chip8.v[0x1], 0x0F);
        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_binary_xor_8xy3() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 0x3C;
        chip8.v[0x1] = 0xFF;
        chip8.v[0xF] = 1;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8013);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0xC3);
        assert_eq!(chip8.v[0x1], 0xFF);
        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_call_error_handling_2nnn() {
        let mut chip8 = Cpu::new();

        chip8.sp = 15;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x2123);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.sp, 16);
        assert_eq!(chip8.stack[15], 0x200);
        assert_eq!(chip8.pc, 0x123);

        assert!(chip8.execute(opcode, x, y, n, nn, nnn).is_err());
    }

    #[test]
    fn test_return_error_handling_00ee() {
        let mut chip8 = Cpu::new();

        chip8.sp = 1;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x00EE);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.sp, 0);
        assert_eq!(chip8.pc, 0);

        assert!(chip8.execute(opcode, x, y, n, nn, nnn).is_err());
    }

    #[test]
    fn test_subtract_8xy5_8xy7() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 0x1;
        chip8.v[0x1] = 0x1;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8005);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0x0);
        assert_eq!(chip8.v[0xF], 1);

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8015);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0xFF);
        assert_eq!(chip8.v[0x1], 0x1);
        assert_eq!(chip8.v[0xF], 0);

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0x8017);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.v[0x0], 0x2);
        assert_eq!(chip8.v[0x1], 0x1);
        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_error_handling_fx29() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 0x0;

        let (opcode, x, y, n, nn, nnn) = Cpu::decode(0xF029);
        chip8.execute(opcode, x, y, n, nn, nnn).unwrap();

        assert_eq!(chip8.i, 0x050);

        chip8.v[0x0] = 0x1F;

        assert!(chip8.execute(opcode, x, y, n, nn, nnn).is_err());
    }

    #[test]
    fn test_skip_if_key_ex9e_exa1() {
        let mut chip8 = Cpu::new();

        chip8.v[0x0] = 0xF;

        // E09E - EX9E
        let (opcode_0, x_0, y_0, n_0, nn_0, nnn_0) = Cpu::decode(0xE09E);
        // E0A1 - EXA1
        let (opcode_1, x_1, y_1, n_1, nn_1, nnn_1) = Cpu::decode(0xE0A1);

        chip8.execute(opcode_0, x_0, y_0, n_0, nn_0, nnn_0).unwrap();

        assert_eq!(chip8.pc, 512);

        chip8.execute(opcode_1, x_1, y_1, n_1, nn_1, nnn_1).unwrap();

        assert_eq!(chip8.pc, 514);

        chip8.keypad[0xF] = true;

        chip8.execute(opcode_0, x_0, y_0, n_0, nn_0, nnn_0).unwrap();

        assert_eq!(chip8.pc, 516);

        chip8.execute(opcode_1, x_1, y_1, n_1, nn_1, nnn_1).unwrap();

        assert_eq!(chip8.pc, 516);

        chip8.v[0x0] = 0x1F;

        assert!(chip8.execute(opcode_0, x_0, y_0, n_0, nn_0, nnn_0).is_err());
        assert!(chip8.execute(opcode_1, x_1, y_1, n_1, nn_1, nnn_1).is_err());
    }
}
