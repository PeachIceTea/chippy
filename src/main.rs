use rand::random;
use raylib::prelude::*;
use std::fs;

const RAM_SIZE: usize = 0x1000;
const REGISTER_COUNT: usize = 16;
const RESERVED_SPACE: usize = 0x200;
const SCREEN_X_SIZE: usize = 64;
const SCREEN_Y_SIZE: usize = 32;
const MAX_ROM_SIZE: usize = 0x1000 - RESERVED_SPACE;
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
const STACK_SIZE: usize = 16;

const SCREEN_SCALE: i32 = 30;
const DEBUG_FONT_SIZE: i32 = 20;

struct Chip8 {
    ready: bool,
    running: bool,

    pc: usize,
    stack: [usize; STACK_SIZE],
    sp: usize,
    vram: [[bool; SCREEN_X_SIZE]; SCREEN_Y_SIZE],

    ram: [u8; RAM_SIZE],
    v: [u8; REGISTER_COUNT],
    i: u16,
}

impl Chip8 {
    fn load_rom(&mut self, path: &str) {
        let rom = fs::read(path).unwrap();

        if rom.len() > MAX_ROM_SIZE {
            eprintln!(
                "The rom is too large. {} bytes, max size is {}.",
                rom.len(),
                MAX_ROM_SIZE
            );
        } else {
            for (i, val) in rom.into_iter().enumerate() {
                self.ram[RESERVED_SPACE + i] = val;
            }
            self.ready = true;
        }
    }

    fn new() -> Chip8 {
        let mut ch = Chip8 {
            ready: false,
            running: true,

            pc: RESERVED_SPACE,
            stack: [0; STACK_SIZE],
            sp: 0,
            vram: [[false; SCREEN_X_SIZE]; SCREEN_Y_SIZE],

            ram: [0; RAM_SIZE],
            v: [0; REGISTER_COUNT],
            i: 0,
        };

        for i in 0..80 {
            ch.ram[i] = FONTSET[i];
        }

        ch
    }

    fn op_00e0(&mut self) -> usize {
        for row in self.vram.iter_mut() {
            row.fill(false);
        }
        self.pc + 2
    }

    fn op_00ee(&mut self) -> usize {
        let jmp = self.stack[self.sp];
        self.sp -= 1;
        jmp
    }

    fn op_1nnn(&mut self, opcode: u16, nnn: u16) -> usize {
        let jmp = nnn as usize;
        if jmp == self.pc {
            eprintln!("Infinite loop at 0x{:X}. Opcode 0x{:X}.", self.pc, opcode);
            self.ready = false;
        }

        jmp
    }

    fn op_2nnn(&mut self, nnn: u16) -> usize {
        if self.pc == STACK_SIZE {
            println!("Stack is full.");
            self.ready = false;
            return self.pc;
        }

        self.sp += 1;
        self.stack[self.sp] = self.pc + 2;

        nnn as usize
    }

    fn op_3xnn(&self, vx: u8, nn: u8) -> usize {
        if self.v[vx as usize] == nn {
            self.pc + 4
        } else {
            self.pc + 2
        }
    }

    fn op_4xnn(&self, vx: u8, nn: u8) -> usize {
        if self.v[vx as usize] != nn {
            self.pc + 4
        } else {
            self.pc + 2
        }
    }

    fn op_5xy0(&self, vx: u8, vy: u8) -> usize {
        if self.v[vx as usize] == self.v[vy as usize] {
            self.pc + 4
        } else {
            self.pc + 2
        }
    }

    fn op_6xnn(&mut self, vx: u8, nn: u8) -> usize {
        self.v[vx as usize] = nn;
        self.pc + 2
    }

    fn op_7xnn(&mut self, vx: u8, nn: u8) -> usize {
        self.v[vx as usize] = self.v[vx as usize].wrapping_add(nn);
        self.pc + 2
    }

    fn op_8xy0(&mut self, vx: u8, vy: u8) -> usize {
        self.v[vx as usize] = self.v[vy as usize];
        self.pc + 2
    }

    fn op_8xy1(&mut self, vx: u8, vy: u8) -> usize {
        self.v[vx as usize] |= self.v[vy as usize];
        self.pc + 2
    }

    fn op_8xy2(&mut self, vx: u8, vy: u8) -> usize {
        self.v[vx as usize] &= self.v[vy as usize];
        self.pc + 2
    }

    fn op_8xy3(&mut self, vx: u8, vy: u8) -> usize {
        self.v[vx as usize] ^= self.v[vy as usize];
        self.pc + 2
    }

    fn op_8xy4(&mut self, vx: u8, vy: u8) -> usize {
        let (result, borrow) = self.v[vx as usize].overflowing_add(self.v[vy as usize]);
        self.v[vx as usize] = result;
        self.v[0xF] = if borrow { 0 } else { 1 };
        self.pc + 2
    }

    fn op_8xy5(&mut self, vx: u8, vy: u8) -> usize {
        let (result, borrow) = self.v[vx as usize].overflowing_sub(self.v[vy as usize]);
        self.v[vx as usize] = result;
        if borrow {
            self.v[0xF] = 0;
        } else {
            self.v[0xF] = 1;
        }
        self.pc + 2
    }

    fn op_8xy6(&mut self, vx: u8, vy: u8) -> usize {
        let tmp = self.v[vy as usize];
        self.v[vx as usize] = tmp >> 1;
        self.v[0xF] = tmp & 1;
        self.pc + 2
    }

    fn op_8xy7(&mut self, vx: u8, vy: u8) -> usize {
        let (result, borrow) = self.v[vy as usize].overflowing_sub(self.v[vx as usize]);
        self.v[vx as usize] = result;
        self.v[0xF] = if borrow { 1 } else { 0 };
        self.pc + 2
    }

    fn op_8xye(&mut self, vx: u8, vy: u8) -> usize {
        let tmp = self.v[vy as usize];
        self.v[vx as usize] = tmp << 1;
        self.v[0xF] = tmp >> 7;
        self.pc + 2
    }

    fn op_9xy0(&self, vx: u8, vy: u8) -> usize {
        if self.v[vx as usize] != self.v[vy as usize] {
            self.pc + 4
        } else {
            self.pc + 2
        }
    }

    fn op_annn(&mut self, nnn: u16) -> usize {
        self.i = nnn;
        self.pc + 2
    }

    fn op_bnnn(&mut self, nnn: u16) -> usize {
        (nnn + (self.v[0] as u16)) as usize
    }

    fn op_cxnn(&mut self, vx: u8, nn: u8) -> usize {
        self.v[vx as usize] = random::<u8>() & nn;
        self.pc + 1
    }

    fn op_dxyn(&mut self, vx: u8, vy: u8, n: u8) -> usize {
        for y_cord in 0..(n as usize) {
            let line = self.ram[self.i as usize + y_cord];
            for x_cord in 0..8 {
                if (line & (0b1000000 >> x_cord)) != 0 {
                    let x = vx as usize + x_cord;
                    let y = vy as usize + y_cord;
                    if self.vram[y][x] {
                        self.v[0xF] = 1;
                    }
                    self.vram[y][x] ^= true;
                }
            }
        }
        self.pc + 2
    }

    fn op_ex9e(&mut self, vx: u8) -> usize {
        todo!()
    }

    fn tick(&mut self) {
        if self.running {
            if self.pc > RAM_SIZE {
                self.ready = false;
                return;
            }

            let opcode = ((self.ram[self.pc] as u16) << 8) | self.ram[self.pc + 1] as u16;
            let nibbels = (
                ((opcode & 0xF000) >> 12) as u8,
                ((opcode & 0x0F00) >> 8) as u8,
                ((opcode & 0x00F0) >> 4) as u8,
                (opcode & 0x000F) as u8,
            );
            let nn = (opcode & 0xFF) as u8;
            let nnn = opcode & 0xFFF;

            eprintln!("0x{:X}: 0x{:X}", self.pc, opcode);

            self.pc = match nibbels {
                (0x0, 0x0, 0xE, 0xE) => self.op_00ee(),
                (0x0, 0x0, 0xE, 0x0) => self.op_00e0(),
                (0x1, _, _, _) => self.op_1nnn(opcode, nnn),
                (0x2, _, _, _) => self.op_2nnn(nnn),
                (0x3, _, _, _) => self.op_3xnn(nibbels.1, nn),
                (0x4, _, _, _) => self.op_4xnn(nibbels.1, nn),
                (0x5, _, _, 0x0) => self.op_5xy0(nibbels.1, nibbels.2),
                (0x6, _, _, _) => self.op_6xnn(nibbels.1, nn),
                (0x7, _, _, _) => self.op_7xnn(nibbels.1, nn),
                (0x8, _, _, 0x0) => self.op_8xy0(nibbels.1, nibbels.2),
                (0x8, _, _, 0x1) => self.op_8xy1(nibbels.1, nibbels.2),
                (0x8, _, _, 0x2) => self.op_8xy2(nibbels.1, nibbels.2),
                (0x8, _, _, 0x3) => self.op_8xy3(nibbels.1, nibbels.2),
                (0x8, _, _, 0x4) => self.op_8xy4(nibbels.1, nibbels.2),
                (0x8, _, _, 0x5) => self.op_8xy5(nibbels.1, nibbels.2),
                (0x8, _, _, 0x6) => self.op_8xy6(nibbels.1, nibbels.2),
                (0x8, _, _, 0x7) => self.op_8xy7(nibbels.1, nibbels.2),
                (0x8, _, _, 0xE) => self.op_8xye(nibbels.1, nibbels.2),
                (0x9, _, _, 0x0) => self.op_9xy0(nibbels.1, nibbels.2),
                (0xA, _, _, _) => self.op_annn(nnn),
                (0xB, _, _, _) => self.op_bnnn(nnn),
                (0xC, _, _, _) => self.op_cxnn(nibbels.1, nn),
                (0xD, _, _, _) => self.op_dxyn(nibbels.1, nibbels.2, nibbels.3),
                (0xE, _, 0x9, 0xE) => self.op_ex9e(nibbels.1),
                _ => {
                    eprintln!("Opcode 0x{:X} not implemented.", opcode);
                    self.running = false;
                    self.pc
                }
            };
        }
    }
}

fn main() {
    let p = "./roms/test-rom.ch8";

    let (mut rl, thread) = raylib::init()
        .size(
            SCREEN_X_SIZE as i32 * SCREEN_SCALE,
            SCREEN_Y_SIZE as i32 * SCREEN_SCALE,
        )
        .title("Chippy")
        .build();
    let mut console = Chip8::new();
    console.load_rom(p);
    rl.set_target_fps(60);

    let mut show_debug = true;
    while !rl.window_should_close() && console.ready {
        console.tick();

        // Input
        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            show_debug = !show_debug;
        }

        // Start print (no input anymore)
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        // Print
        for y in 0..(SCREEN_Y_SIZE - 1) {
            for x in 0..(SCREEN_X_SIZE - 1) {
                if console.vram[y][x] {
                    d.draw_rectangle(
                        x as i32 * SCREEN_SCALE,
                        y as i32 * SCREEN_SCALE,
                        SCREEN_SCALE,
                        SCREEN_SCALE,
                        Color::WHITE,
                    );
                }
            }
        }

        // Debug print
        if show_debug {
            d.draw_rectangle(
                0,
                0,
                DEBUG_FONT_SIZE * 10,
                DEBUG_FONT_SIZE * 20,
                Color::BLACK,
            );
            d.draw_text(
                format!("FPS: {}", d.get_fps()).as_str(),
                0,
                0,
                DEBUG_FONT_SIZE,
                Color::WHITE,
            );

            d.draw_text(
                format!("I: 0x{:X}", console.i).as_str(),
                0,
                DEBUG_FONT_SIZE * 2,
                DEBUG_FONT_SIZE,
                Color::WHITE,
            );
            for i in 0..(REGISTER_COUNT) {
                d.draw_text(
                    format!("V{:X}: 0x{:X}", i, console.v[i]).as_str(),
                    0,
                    (DEBUG_FONT_SIZE * 4) + (i as i32 * DEBUG_FONT_SIZE),
                    DEBUG_FONT_SIZE,
                    Color::WHITE,
                );
            }
        }
    }
}
