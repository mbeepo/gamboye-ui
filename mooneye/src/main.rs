use std::{env::{args, current_dir}, fs::read, path::PathBuf};

use gbc::CpuStatus;

fn main() {
    let path: PathBuf = args().nth(1).unwrap_or("roms".to_owned()).into();
    let path: PathBuf = if path.is_relative() {
        [current_dir().unwrap(), path].iter().collect()
    } else {
        path
    };

    if path.exists() {
        if path.is_file() {
            if let Ok(_) = run_test(path) {
                println!("Test passed");
            } else {
                println!("[X] Test failed [X]");
            }
        } else if path.is_dir() {
            for entry in path.read_dir().unwrap().filter(
                |entry| entry.as_ref().is_ok_and(
                    |entry| entry.file_name().to_str().is_some_and(
                        |name| name.ends_with(".gb")
                    )
                )
            ) {
                if let Ok(_) = run_test(entry.unwrap().path()) {
                    println!("Test passed");
                } else {
                    println!("[X] Test failed [X]");
                }
            }
        }
    } else {
        println!("Path not found: {path:?}");
    }
}

fn run_test(path: PathBuf) -> Result<(), ()> {
    println!("Running test {:?}", path.file_name().unwrap());
    let rom = read(path).unwrap();
    let mbc = gbc::get_mbc(&rom);
    let mut sys = gbc::Gbc::new(mbc, false, true);

    sys.cpu.breakpoint_controls.set(gbc::CpuEvent::LdBb);
    sys.load_rom(&rom);
    sys.disable_ppu();

    while let Ok(status) = sys.step().0 {
        match status {
            CpuStatus::Break(_, _) => {
                check_reg(sys.cpu.regs.b, 3)?;
                check_reg(sys.cpu.regs.c, 5)?;
                check_reg(sys.cpu.regs.d, 8)?;
                check_reg(sys.cpu.regs.e, 13)?;
                check_reg(sys.cpu.regs.h, 21)?;
                check_reg(sys.cpu.regs.l, 34)?;
                break;
            },
            CpuStatus::Run(_) => {},
            status => panic!("Epic test fail !! {:?}", status),
        }
    }

    Ok(())
}

fn check_reg(reg: u8, value: u8) -> Result<(), ()> {
    if reg == value { Ok(()) }
    else { Err(()) }
}