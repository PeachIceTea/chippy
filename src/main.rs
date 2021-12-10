use raylib::prelude::*;
use std::{fs, thread, time};

const RAM_SIZE: usize = 0x1000;
const REGISTER_COUNT: usize = 16;
const RESERVED_SPACE: usize = 0x200;
const SCREEN_X_SIZE: usize = 64;
const SCREEN_Y_SIZE: usize = 32;
const VRAM_SIZE: usize = SCREEN_X_SIZE * SCREEN_Y_SIZE;

const SCREEN_SCALE: i32 = 20;
const MAX_ROM_SIZE: usize = 0x1000 - RESERVED_SPACE;
const FONT: [u8; 80] = [
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

struct Chip8 {
    ready: bool,
    running: bool,

    pc: usize,

    ram: [u8; RAM_SIZE],
    v: [u8; REGISTER_COUNT],
    i: u16,
}

impl Chip8 {
    fn new() -> Chip8 {
        let mut ch = Chip8 {
            ready: false,
            running: true,

            pc: RESERVED_SPACE,

            ram: [0; RAM_SIZE],
            v: [0; REGISTER_COUNT],
            i: 0,
        };

        for i in 0..80 {
            ch.ram[i] = FONT[i];
        }

        ch
    }

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
                (0x1, _, _, _) => self.op_1nnn(opcode, nnn),
                (0x3, _, _, _) => self.op_3xnn(nibbels.1, nn),
                (0x4, _, _, _) => self.op_4xnn(nibbels.1, nn),
                (0x5, _, _, 0) => self.op_5xy0(nibbels.1, nibbels.2),
                (0x6, _, _, _) => self.op_6xnn(nibbels.1, nn),
                (0x7, _, _, _) => self.op_7xnn(nibbels.1, nn),
                (0xA, _, _, _) => self.op_annn(nnn),
                (0xD, _, _, _) => self.op_dxyn(opcode, nibbels.1, nibbels.2, nibbels.3),
                _ => {
                    eprintln!("Opcode 0x{:X} not implemented.", opcode);
                    self.running = false;
                    self.pc
                }
            };
        }
    }

    fn op_1nnn(&mut self, opcode: u16, nnn: u16) -> usize {
        let jmp = nnn as usize;
        if jmp == self.pc {
            eprintln!("Infinite loop at 0x{:X}. Opcode 0x{:X}.", self.pc, opcode);
            self.ready = false;
        }

        jmp
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
        self.v[vx as usize] = self.v[vx as usize] + nn;
        self.pc + 2
    }

    fn op_annn(&mut self, nnn: u16) -> usize {
        self.i = nnn;
        self.pc + 2
    }

    fn op_dxyn(&self, opcode: u16, _vx: u8, _vy: u8, _n: u8) -> usize {
        println!("TODO: Print to screen 0x{:X}", opcode);

        self.pc + 2
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
        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            show_debug = !show_debug;
        }

        console.tick();
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        if show_debug {
            d.draw_text(
                format!("FPS: {}", d.get_fps()).as_str(),
                0,
                0,
                12,
                Color::WHITE,
            );

            d.draw_text(
                format!("I: 0x{:X}", console.i).as_str(),
                0,
                12 * 2,
                12,
                Color::WHITE,
            );
            for i in 0..(REGISTER_COUNT - 1) {
                d.draw_text(
                    format!("V{:X}: 0x{:X}", i, console.v[i]).as_str(),
                    0,
                    (12 * 4) + (i * 12) as i32,
                    12,
                    Color::WHITE,
                );
            }
        }
    }
}
