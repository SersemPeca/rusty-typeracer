
use std::{
    fmt::Display,
    io::{stdout, Stdout, Write},
};

use termion::{
    clear,
    color::{self, Color},
    cursor::{self, DetectCursorPos},
    raw::{IntoRawMode, RawTerminal},
    style, terminal_size,
};

use crate::GameError;
use crate::text::{
    Text,
    HasLength,
};

/// We will use this to format lines adequately
#[derive(Clone, Copy)]
struct Pos {
    pub x: u16,
    pub y: u16,
    pub len: u16,
}

struct Cursor {
    pub lines: Vec<Pos>,
    pub curr_line: usize,
    pub curr_char: u16,
}

const MIN_LINE_WIDTH: usize = 50;

impl Cursor {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            curr_line: 0,
            curr_char: 0,
        }
    }

    pub fn cur_pos(&self) -> (u16, u16) {
        let line = self.lines[self.curr_line];
        (line.x + self.curr_char, line.y) 
    }

    pub fn next(&mut self) -> (u16, u16) {
        let line = self.lines[self.curr_line];
        let last_index = line.len - 1;

        if self.curr_char < last_index {
            self.curr_char += 1;
        }
        else { /// We must move one line down (If we can)
            if self.curr_line + 1 < self.lines.len() {
                self.curr_line += 1;
                self.curr_char = 0;
            }
        }

        self.cur_pos()
    }

    pub fn prev(&mut self) -> (u16, u16) {

        if self.curr_char > 0 {
            self.curr_char -= 1;
        } else {
            if self.curr_line > 0 {
                self.curr_line -= 1;
                let line = self.lines[self.curr_line];

                self.curr_char = line.len - 1;
            }
        }

        self.cur_pos()
    }

}

pub struct GameTui {
    stdout: RawTerminal<Stdout>,
    cursor: Cursor,
    track_lines: bool,
}

impl GameTui {
    pub fn new() -> Self {
        Self {
            stdout: stdout().into_raw_mode().unwrap(),
            cursor: Cursor::new(),
            track_lines: false,
        }
    }

    pub fn flush(&mut self) -> Result<(), GameError> {
        self.stdout.flush()?;
        Ok(())

    }
    /// Resets terminal.
    ///
    /// Clears screen and sets the cursor to a non-blinking block.
    pub fn reset(&mut self) {
        self.cursor = Cursor::new();
    }

    pub fn reset_screen(&mut self) -> Result<(), GameError> {
        let (sizex, sizey) = terminal_size()?;

        write!(
            self.stdout,
            "{}{}{}",
            clear::All,
            cursor::Goto(sizex / 2, sizey / 2),
            cursor::BlinkingBar
            )?;

        self.flush()?;

        Ok(())
    }

    fn print_line_raw<T, U>(&mut self, text: U) -> Result<(), GameError>
        where 
        U: AsRef<[T]>,
    [T]: HasLength,
    T: Display, {
        let len = text.as_ref().length() as u16;
        write!(self.stdout, "{}", cursor::Left(len / 2))?;


        if self.track_lines {
            let (x, y) = self.stdout.cursor_pos()?;
            self.cursor.lines.push(Pos {x, y, len: len});
        }

        for t in text.as_ref() {
            self.print_text_raw(t)?;
        }
        /// TODO: Change ?
        write!(self.stdout, "{}", cursor::Left(len))?;
        Ok(())
    }

    pub fn print_line(&mut self, text: &[Text]) -> Result<(), GameError> {
        self.print_line_raw(text)?;
        self.flush()?;

        Ok(())
    }

    pub fn print_lines<T, U>(&mut self, lines: &[T]) -> Result<(), GameError>
        where
        T: AsRef<[U]>,
    [U]: HasLength,
    U: Display,
    {
        let (sizex, sizey) = terminal_size()?;

        let line_offset = lines.len() as u16 / 2;

        for (line_no, line) in lines.iter().enumerate() {
            write!(
                self.stdout,
                "{}",
                cursor::Goto(sizex / 2, sizey / 2 + (line_no as u16) - line_offset)
                )?;
            self.print_line_raw(line.as_ref())?;
        }
        self.flush()?;

        Ok(())
    }

    /// TODO: Display the keys that terminate the program

    pub fn print_text_raw<T>(&mut self, text: &T) -> Result<(), GameError>
        where
        T: Display,
        {
            write!(self.stdout, "{}", text)?;
            Ok(())
        }

    pub fn hide_cursor(&mut self) ->  Result<(), GameError> {
        write!(self.stdout, "{}", cursor::Hide)?;
        self.flush()?;
        Ok(())
    }

    pub fn show_cursor(&mut self) -> Result<(), GameError> {
        write!(self.stdout, "{}", cursor::Show)?;
        self.flush()?;
        Ok(())
    }

    pub fn display_words(&mut self, words: &[String]) -> Result<Vec<Text>, GameError> {
        self.reset();
        let mut current_len = 0;
        let mut max_word_len = 0;
        let mut line = Vec::new();
        let mut lines = Vec::new();
        let (terminal_width, terminal_height) = terminal_size()?;
        // 40% of terminal width
        let max_width = terminal_width * 2 / 5;
        const MAX_WORDS_PER_LINE: usize = 10;
        // eprintln!("max width is {}", max_width);

        for word in words {
            max_word_len = std::cmp::max(max_word_len, word.len() + 1);
            /// Characters      Whitespace
            let new_len = current_len + word.len() as u16 + 1;
            if line.len() < MAX_WORDS_PER_LINE && new_len <= max_width {
                // add to line
                line.push(word.clone());
                current_len += word.len() as u16 + 1
            } else {
                /// Add extra whitespace at the end of every word
                lines.push(Text::from(line.join(" ") + " ").with_faint());

                line = vec![word.clone()];
                current_len = word.len() as u16 + 1;
            }
        }

        /// Logic for adding the last line
        lines.push(Text::from(line.join(" ")).with_faint());

        max_word_len = std::cmp::max(max_word_len + 1, MIN_LINE_WIDTH);
        if lines.len() /*+ self.bottom_lines_len*/ + 2 > terminal_height as usize {
            return Err(GameError::from(format!(
                        "Terminal height is too short! Toipe requires at least {} lines, got {} lines",
                        lines.len() /*+ self.bottom_lines_len */+ 2,
                        terminal_height,
                        )));
        } else if max_word_len > terminal_width as usize {
            return Err(GameError::from(format!(
                        "Terminal width is too low! Toipe requires at least {} columns, got {} columns",
                        max_word_len, terminal_width,
                        )));
        }

        self.track_lines = true;
        self.print_lines(
            lines
            .iter()
            .cloned()
            .map(|line| [line])
            .collect::<Vec<[Text; 1]>>()
            .as_slice(),
            )?;
        self.track_lines = false;

        self.move_to_cur_pos()?;
        self.flush()?;

        Ok(lines)
    }

    pub fn replace_text<T>(&mut self, text: T) -> Result<(), GameError>
        where
        T: Display,
        {
            self.move_to_prev_char()?;
            self.print_text_raw(&text)?;
            self.move_to_cur_pos()?;

            Ok(())
        }

    /// Moves the cursor to the next char
    pub fn move_to_next_char(&mut self) -> Result<(), GameError> {
        let (x, y) = self.cursor.next();
        write!(self.stdout, "{}", cursor::Goto(x, y))?;

        Ok(())
    }

    /// Moves the cursor to the previous char
    pub fn move_to_prev_char(&mut self) -> Result<(), GameError> {
        let (x, y) = self.cursor.prev();
        write!(self.stdout, "{}", cursor::Goto(x, y))?;

        Ok(())
    }

    /// Moves the cursor to just before the character to be typed next
    pub fn move_to_cur_pos(&mut self) -> Result<(), GameError> {
        let (x, y) = self.cursor.cur_pos();
        write!(self.stdout, "{}", cursor::Goto(x, y))?;

        Ok(())
    }

    /// Returns the current line the cursor is on
    pub fn current_line(&self) -> usize {
        self.cursor.curr_line
    }
}

impl Default for GameTui {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GameTui {
    ///
    /// TODO: print error message when terminal height/width is too small.
    /// Take a look at https://github.com/Samyak2/toipe/pull/28#discussion_r851784291 for more info.
    fn drop(&mut self) {
        write!(
            self.stdout,
            "{}{}{}",
            clear::All,
            cursor::SteadyBlock,
            cursor::Goto(1, 1)
            )
            .expect("Could not reset terminal while exiting");
        self.flush().expect("Could not flush stdout while exiting");
    }

}
