extern crate termion;

use termion::{color, cursor, clear, terminal_size};
use termion::raw::IntoRawMode;

use std::fmt;
use std::io::{Write, stdout, stdin};

use playgorund::{
    GameError,
    Game,
};

#[derive(Clone)]
struct Text {
    pub raw_text: String,
    pub formatted_text: String,
    pub len: usize,
}

impl Text {
    pub fn new(str: String) -> Self {
        Text {
            raw_text: str.clone(),
            formatted_text: str.clone(),
            len: str.len(),
        }
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.formatted_text)
    }
}

fn main() -> Result<(), GameError> {

    let stdin = stdin();

    let mut game = Game::new()?;

    loop {
        let stdin = stdin.lock();
        if let Ok((true, _)) = game.run(stdin) {
            game.restart()?;
        } else {
            break;
        }
    }
    Ok(())
}
