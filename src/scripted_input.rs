use bracket_terminal::prelude::VirtualKeyCode;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

pub struct ScriptedInput {
    script_commands: Vec<VirtualKeyCode>,
    current_command_index: usize,
}

impl ScriptedInput {
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut script_commands = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                continue; // Skip empty lines and comments
            }

            if let Some(key) = string_to_virtualkeycode(trimmed_line) {
                script_commands.push(key);
            } else {
                eprintln!("Warning: Unknown command in script: {}", trimmed_line);
            }
        }

        Ok(Self {
            script_commands,
            current_command_index: 0,
        })
    }

    pub fn next_key(&mut self) -> Option<VirtualKeyCode> {
        if self.current_command_index < self.script_commands.len() {
            let key = self.script_commands[self.current_command_index];
            self.current_command_index += 1;
            Some(key)
        } else {
            None
        }
    }
}

fn string_to_virtualkeycode(s: &str) -> Option<VirtualKeyCode> {
    match s.to_lowercase().as_str() {
        "up" | "k" | "w" => Some(VirtualKeyCode::Up),
        "down" | "j" | "s" => Some(VirtualKeyCode::Down),
        "left" | "h" | "a" => Some(VirtualKeyCode::Left),
        "right" | "l" | "d" => Some(VirtualKeyCode::Right),
        "ascend" | "<" => Some(VirtualKeyCode::PageDown),
        "descend" | ">" => Some(VirtualKeyCode::PageUp),
        "cycle" | "tab" => Some(VirtualKeyCode::Tab),
        "cycle_rev" | "backtab" => Some(VirtualKeyCode::Back),
        "item1" | "1" => Some(VirtualKeyCode::Key1),
        "item2" | "2" => Some(VirtualKeyCode::Key2),
        "item3" | "3" => Some(VirtualKeyCode::Key3),
        "item4" | "4" => Some(VirtualKeyCode::Key4),
        "reset" | "r" => Some(VirtualKeyCode::R),
        "quit" | "q" | "escape" => Some(VirtualKeyCode::Escape),
        "wait" | "." => Some(VirtualKeyCode::Period),
        "turn" | "t" => Some(VirtualKeyCode::T),
        "dump" | "p" => Some(VirtualKeyCode::P),
        _ => None,
    }
}