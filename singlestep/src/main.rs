use std::{env::args, fs::{read, read_dir, DirEntry, File}, io::{stdout, Read, Write}, path::{Path, PathBuf}};

use gbc::{CpuError, CpuStatus, Gbc, Mmu};
use serde::{de::{self, Visitor}, Deserialize};

#[derive(Deserialize, Debug)]
struct Test {
    name: String,
    initial: State,
    #[serde(rename = "final")]
    _final: State,
    cycles: Vec<Cycle>,
}

#[derive(Deserialize, Debug)]
struct State {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
    ime: u8,
    ie: Option<u8>,
    ram: Vec<MemCell>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
struct MemCell {
    addr: u16,
    value: u8,
}

#[derive(Deserialize, Clone, Copy, Debug)]
struct Cycle(u16, Option<u8>, MemoryMode);

#[derive(Clone, Copy, Debug)]
struct MemoryMode {
    read: bool,
    write: bool,
    request: bool,
}

impl<'de> Deserialize<'de> for MemoryMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        struct MemoryModeVisitor;

        impl<'de> Visitor<'de> for MemoryModeVisitor {
            type Value = MemoryMode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("string 'rwm', where any character may be a '-'")
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
                where
                    E: de::Error, {
                let read = v.chars().nth(0)
                        .and_then(|r: char| if r != '-' { Some(true) } else { Some(false) })
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let write = v.chars().nth(1)
                        .and_then(|r: char| if r != '-' { Some(true) } else { Some(false) })
                        .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let request = v.chars().nth(2)
                        .and_then(|r: char| if r != '-' { Some(true) } else { Some(false) })
                        .ok_or_else(|| de::Error::invalid_length(2, &self))?;

                Ok(MemoryMode { read, write, request })
            }
        }

        deserializer.deserialize_str(MemoryModeVisitor)
    }
}

fn main() {
    let mut sys = Gbc::new(gbc::MbcSelector::NoMbc, false, true);
    let files: Vec<DirEntry> = if let Some(instruction) = args().nth(1) {
        let mut path: PathBuf = [".", "sm83", "v1", &instruction].iter().collect();
        path.set_extension("json");
        let buf = read(path).unwrap();

        run_tests(&mut sys, buf).unwrap();
        return;
    } else { 
        let files = read_dir("sm83/v1").unwrap();
        files.map(|e| e.unwrap()).collect() 
    };

    let mut failures: Vec<(String, String)> = Vec::with_capacity(8);

    for entry in files {
        let buf = read(entry.path()).unwrap();
        if let Err(err) = run_tests(&mut sys, buf) {
            failures.push(err);
        }
    }

    for (test, failure) in failures {
        println!("Test {test} failed: {failure}");
    }
}

fn run_tests(sys: &mut Gbc, buf: Vec<u8>) -> Result<(), (String, String)> {
    let tests: Vec<Test> = serde_json::from_slice(&buf).unwrap();

    println!("Test {}", tests[0].name);

    for test in tests {
        init_state(sys, test.initial);
        sys.step().0.unwrap();

       assert_state(&sys, test._final).map_err(|err| (test.name, err))?;
    }

    Ok(())
}

fn init_state(sys: &mut Gbc, state: State) {
    sys.cpu.regs.a = state.a;
    sys.cpu.regs.b = state.b;
    sys.cpu.regs.c = state.c;
    sys.cpu.regs.d = state.d;
    sys.cpu.regs.e = state.e;
    sys.cpu.regs.f = gbc::Flags::new();
    sys.cpu.regs.f.set_bits(state.f);
    sys.cpu.regs.h = state.h;
    sys.cpu.regs.l = state.l;
    sys.cpu.regs.pc = state.pc;
    sys.cpu.regs.sp = state.sp;
    sys.cpu.regs.ime = state.ime == 1;
    sys.cpu.ppu.lcdc.lcd_enable = false;
    sys.cpu.div = 0;

    sys.cpu.memory = Box::new(Mmu::new(gbc::MbcSelector::NoMbc));
    
    sys.cpu.memory.splice(0xff00, &[0; 0x80]);

    if let Some(ie) = state.ie {
        sys.cpu.memory.set(gbc::memory::IE, ie);
    }

    for cell in state.ram {
        sys.cpu.memory.set(cell.addr, cell.value);
        
        if cell.addr == gbc::memory::DIV { sys.cpu.div = (cell.value as u16) << 8 };
        assert_eq!(sys.cpu.memory.load(cell.addr).unwrap(), cell.value);
    }
}

fn assert_state(sys: &Gbc, state: State) -> Result<(), String> {
    for cell in state.ram {
        let value = sys.cpu.memory.load(cell.addr).unwrap();
        if value != cell.value { return Err(format!("[{}] = {}, expected {}", cell.addr, value, cell.value)) };
    }

    if sys.cpu.regs.a != state.a { Err(format!("A = {}, expected {}", sys.cpu.regs.a, state.a)) }
    else if sys.cpu.regs.b != state.b { Err(format!("B = {}, expected {}", sys.cpu.regs.b, state.b)) }
    else if sys.cpu.regs.c != state.c { Err(format!("C = {}, expected {}", sys.cpu.regs.c, state.c)) }
    else if sys.cpu.regs.d != state.d { Err(format!("D = {}, expected {}", sys.cpu.regs.d, state.d)) }
    else if sys.cpu.regs.e != state.e { Err(format!("E = {}, expected {}", sys.cpu.regs.e, state.e)) }
    else if sys.cpu.regs.f.as_byte() != state.f {  Err(format!("F = {}, expected {}", sys.cpu.regs.f.as_byte(), state.f)) }
    else if sys.cpu.regs.h != state.h { Err(format!("H = {}, expected {}", sys.cpu.regs.h, state.h)) }
    else if sys.cpu.regs.l != state.l { Err(format!("L = {}, expected {}", sys.cpu.regs.l, state.l)) }
    else if sys.cpu.regs.pc != state.pc { Err(format!("PC = {}, expected {}", sys.cpu.regs.pc, state.pc)) }
    else if sys.cpu.regs.sp != state.sp { Err(format!("SP = {}, expected {}", sys.cpu.regs.sp, state.sp)) }
    else if sys.cpu.regs.ime != (state.ime == 1) { Err(format!("IME = {}, expected {}", sys.cpu.regs.ime, state.ime == 1)) }
    else if let Some(ie) = state.ie {
        let sys_ie = sys.cpu.memory.load(gbc::memory::IE).unwrap();
        if sys_ie != ie { Err(format!("IE = {:#08X}, expected {}", sys_ie, ie)) }
        else { Ok(()) }
    } else {
        Ok(())
    }
}