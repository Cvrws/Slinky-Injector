extern crate winapi;

use crossterm::{
    cursor::MoveTo,
    event::{self, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{self, Write};
use std::ptr::null_mut;
use std::time::Duration;
use winapi::shared::minwindef::DWORD;
use winapi::um::winuser::{
    FindWindowA, FindWindowExA, GetWindowTextA, GetWindowTextLengthA, GetWindowThreadProcessId,
};

#[derive(Clone)]
pub struct MinecraftInstance {
    pub pid: u32,
    pub title: String,
}

pub struct MinecraftUtil;

impl MinecraftUtil {
    pub fn get_minecraft_instances() -> Vec<MinecraftInstance> {
        let mut instances = Vec::new();
        let mut hwnd = unsafe { FindWindowA("LWJGL\0".as_ptr() as *const i8, null_mut()) };

        while !hwnd.is_null() {
            let mut pid: DWORD = 0;
            unsafe {
                GetWindowThreadProcessId(hwnd, &mut pid);
            }

            let length = unsafe { GetWindowTextLengthA(hwnd) } as usize;
            if length > 0 {
                let mut buffer = vec![0u8; length + 1];
                unsafe {
                    GetWindowTextA(hwnd, buffer.as_mut_ptr() as *mut i8, length as i32 + 1);
                }
                let title = String::from_utf8_lossy(&buffer).trim_end_matches('\0').to_string();

                instances.push(MinecraftInstance { pid, title });
            }

            hwnd = unsafe { FindWindowExA(null_mut(), hwnd, "LWJGL\0".as_ptr() as *const i8, null_mut()) };
        }

        instances
    }

    pub fn select_instance(_instances: &[MinecraftInstance]) -> Option<u32> {
        enable_raw_mode().unwrap();
        let mut stdout = io::stdout();
    
        let mut index = 0;
        let mut last_instances = Vec::new();
    
        loop {
            let instances = MinecraftUtil::get_minecraft_instances();
            
            if instances.is_empty() {
                disable_raw_mode().unwrap();
                return None;
            }
    
            if instances.len() != last_instances.len() {
                execute!(stdout, MoveTo(0, 0), Clear(ClearType::All)).unwrap();
                println!("Select an instance to inject (use ↑ ↓ and Enter):\n");
                last_instances = instances.clone();
            }
    
            for (i, instance) in instances.iter().enumerate() {
                execute!(stdout, MoveTo(0, (i + 2) as u16)).unwrap();
                if i == index {
                    print!(" ➜ [{}] {}", i + 1, instance.title);
                } else {
                    print!("   [{}] {}", i + 1, instance.title);
                }
            }
    
            stdout.flush().unwrap();
    
            if let Ok(true) = event::poll(Duration::from_millis(200)) {
                if let Ok(event::Event::Key(key_event)) = event::read() {
                    match key_event.code {
                        KeyCode::Up => {
                            if index > 0 {
                                index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if index < instances.len().saturating_sub(1) {
                                index += 1;
                            }
                        }
                        KeyCode::Enter => {
                            disable_raw_mode().unwrap();
                            execute!(stdout, MoveTo(0, 0), Clear(ClearType::All)).unwrap();
                            return Some(instances[index].pid);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}