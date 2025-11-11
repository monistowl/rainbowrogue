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

            // Parse each character in the line as a keypress
            for char_code in trimmed_line.chars() {
                if let Some(key) = char_to_virtualkeycode(char_code) {
                    script_commands.push(key);
                } else {
                    eprintln!("Warning: Unknown key in script: {}", char_code);
                }
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

fn char_to_virtualkeycode(c: char) -> Option<VirtualKeyCode> {
    match c {
        'w' | 'W' => Some(VirtualKeyCode::W),
        'a' | 'A' => Some(VirtualKeyCode::A),
        's' | 'S' => Some(VirtualKeyCode::S),
        'd' | 'D' => Some(VirtualKeyCode::D),
        'h' | 'H' => Some(VirtualKeyCode::H),
        'j' | 'J' => Some(VirtualKeyCode::J),
        'k' | 'K' => Some(VirtualKeyCode::K),
        'l' | 'L' => Some(VirtualKeyCode::L),
        '<' => Some(VirtualKeyCode::PageUp), // Ascend
        '>' => Some(VirtualKeyCode::PageDown), // Descend
        '\t' => Some(VirtualKeyCode::Tab), // Cycle world forward
        '!' => Some(VirtualKeyCode::Back), // Cycle world backward (using '!' as a placeholder for Backspace/Shift+Tab)
        '1' => Some(VirtualKeyCode::Key1),
        '2' => Some(VirtualKeyCode::Key2),
        '3' => Some(VirtualKeyCode::Key3),
        '4' => Some(VirtualKeyCode::Key4),
        'r' | 'R' => Some(VirtualKeyCode::R),
        'q' | 'Q' => Some(VirtualKeyCode::Escape), // Explicit QUIT command
        '.' => Some(VirtualKeyCode::Period), // Explicit WAIT command
        't' | 'T' => Some(VirtualKeyCode::T), // Step Turn command
        'p' | 'P' => Some(VirtualKeyCode::P), // Dump State command
        '\x1B' => Some(VirtualKeyCode::Escape), // Escape character
        _ => None,
    }
}
