use std::io;

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
}
