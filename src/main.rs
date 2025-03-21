extern crate winapi;

use crossterm::{cursor::MoveTo, execute, terminal::{Clear, ClearType}};
use winapi::um::wincon::GetConsoleWindow;
use winapi::um::winuser::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WS_MAXIMIZEBOX};
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::{VirtualAllocEx, WriteProcessMemory};
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PROCESS_ALL_ACCESS};
use winapi::um::libloaderapi::{GetProcAddress, GetModuleHandleA};
use std::ptr::null_mut;
use std::fs;
use std::env;
use std::process::Command;
use std::ffi::CString;
use std::path::PathBuf;
use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;
use colored::*;
use sysinfo::System;

mod util;
use util::minecraft_util::MinecraftUtil;

const SLINKY_LIBRARY: &[u8] = include_bytes!("../assets/slinky_library.dll");
const SLINKY_HOOK: &[u8] = include_bytes!("../assets/slinkyhook.dll");

fn extract_dlls_if_needed() -> (PathBuf, PathBuf) {
    let exe_dir = env::current_exe()
        .expect("Failed to get executable directory")
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf();
    let hook_path = exe_dir.join("slinkyhook.dll");
    let library_path = exe_dir.join("slinky_library.dll");

    if !hook_path.exists() {
        print!("\rExtracting DLL: {}", hook_path.display());
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(1000));
        fs::write(&hook_path, SLINKY_HOOK).expect("Failed to extract slinkyhook.dll");
        println!("\rExtracting DLL: {} ", hook_path.display().to_string().green());
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(1000));
    }

    if !library_path.exists() {
        print!("\rExtracting 2 DLL: {}", library_path.display());
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(1000));
        fs::write(&library_path, SLINKY_LIBRARY).expect("Failed to extract slinky_library.dll");
        println!("\rExtracting 2 DLL: {} ", library_path.display().to_string().green());
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(1000));
    }

    (hook_path, library_path)
}

fn inject_dll(pid: u32, dll_path: &str) -> bool {
    unsafe {
        let h_process = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if h_process.is_null() {
            return false;
        }

        let dll_path_c = CString::new(dll_path).unwrap();
        let alloc = VirtualAllocEx(
            h_process,
            null_mut(),
            dll_path_c.to_bytes().len() + 1,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if alloc.is_null() {
            CloseHandle(h_process);
            return false;
        }

        let mut written = 0;
        if WriteProcessMemory(
            h_process,
            alloc,
            dll_path_c.as_ptr() as *const _,
            dll_path_c.to_bytes().len() + 1,
            &mut written,
        ) == 0
        {
            CloseHandle(h_process);
            return false;
        }

        let h_kernel32 = GetModuleHandleA("kernel32.dll\0".as_ptr() as *const i8);
        if h_kernel32.is_null() {
            CloseHandle(h_process);
            return false;
        }

        let load_library = GetProcAddress(h_kernel32, "LoadLibraryA\0".as_ptr() as *const i8);
        if load_library.is_null() {
            CloseHandle(h_process);
            return false;
        }

        let h_thread = winapi::um::processthreadsapi::CreateRemoteThread(
            h_process,
            null_mut(),
            0,
            Some(std::mem::transmute(load_library)),
            alloc,
            0,
            null_mut(),
        );

        if h_thread.is_null() {
            CloseHandle(h_process);
            return false;
        }

        CloseHandle(h_thread);
        CloseHandle(h_process);
        true
    }
}

fn main() {
    clear_screen();
    no_resize();

    print!("\x1b]0; \x07");
    io::stdout().flush().unwrap();

    print!("\x1b[?25l");
    io::stdout().flush().unwrap();

    let mut system = System::new_all();
    let mut dot_count = 0;

    loop {
        system.refresh_all();
        let instances = MinecraftUtil::get_minecraft_instances();

        if instances.is_empty() {
            clear_screen();
            execute!(io::stdout(), MoveTo(0, 1), Clear(ClearType::CurrentLine)).unwrap();
            let dots = match dot_count % 4 {
                0 => ".  ",
                1 => ".. ",
                2 => "...",
                _ => "   ",
            };

            print!("\rWaiting for Minecraft process{}", dots.white());
            io::stdout().flush().unwrap();
            dot_count += 1;
        } else {
            execute!(io::stdout(), MoveTo(0, 1), Clear(ClearType::CurrentLine)).unwrap();
            println!("Select an instance to inject (use ↑ ↓ and Enter):");

            if let Some(selected_pid) = MinecraftUtil::select_instance(&instances) {
                let selected_instance = instances.iter().find(|x| x.pid == selected_pid);

                if let Some(instance) = selected_instance {
                    let (hook_path, library_path) = extract_dlls_if_needed();

                    println!("\rInjecting into {}", instance.title.bright_yellow());
                    io::stdout().flush().unwrap();
                    sleep(Duration::from_secs(2));

                    let mut success = true;
                    if !inject_dll(instance.pid, hook_path.to_str().unwrap()) {
                        success = false;
                    }
                    if !inject_dll(instance.pid, library_path.to_str().unwrap()) {
                        success = false;
                    }

                    if success {
                        println!("\r{}", "Successfully injected!".green());
                        io::stdout().flush().unwrap();
                        
                        for i in (0..=5).rev() {
                            print!("\rThis window will close in {}", i);
                            io::stdout().flush().unwrap();
                            sleep(Duration::from_secs(1));
                        }
                    } else {
                        println!("\r{}", "Injection failed!".red());
                    }

                    break;
                }
            }
        }

        sleep(Duration::from_millis(500));
    }
}

fn no_resize() {
    let hwnd = unsafe { GetConsoleWindow() };

    if hwnd != null_mut() {
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };

        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !(WS_MAXIMIZEBOX as isize)) as isize) };
    }
}

fn clear_screen() {
    Command::new("cmd").arg("/C").arg("cls").status().unwrap();
}
