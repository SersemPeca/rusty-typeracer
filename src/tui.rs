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

use crate::Text;
use crate::GameError;
use crate::text::HasLength;

const MIN_LINE_WIDTH: usize = 50;


#[derive(Clone, Copy)]
struct LinePos {

    pub y: u16,

    pub x: u16,

    pub length: u16,
}


struct CursorPos {
    pub lines: Vec<LinePos>,
    pub cur_line: usize,
    pub cur_char_in_line: u16,
}

impl CursorPos {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            cur_line: 0,
            cur_char_in_line: 0,
        }
    }

    pub fn next(&mut self) -> (u16, u16) {
        let line = self.lines[self.cur_line];
        let max_chars_index = line.length - 1;

        if self.cur_char_in_line < max_chars_index {
            // more chars in line
            self.cur_char_in_line += 1;
        } else {
            // reached the end of line
            if self.cur_line + 1 < self.lines.len() {
                // more lines available
                self.cur_line += 1;
                self.cur_char_in_line = 0;
            }
        }

        self.cur_pos()
    }

    pub fn prev(&mut self) -> (u16, u16) {
        if self.cur_char_in_line > 0 {
            // more chars behind in line
            self.cur_char_in_line -= 1;
        } else {
            // reached the start of line
            if self.cur_line > 0 {
                // more lines available
                self.cur_line -= 1;
                self.cur_char_in_line = self.lines[self.cur_line].length - 1;
            }
        }

        self.cur_pos()
    }

    pub fn cur_pos(&self) -> (u16, u16) {
        let line = self.lines[self.cur_line];
        (line.x + self.cur_char_in_line, line.y)
    }
}


pub struct GameTui {
    stdout: RawTerminal<Stdout>,
    cursor_pos: CursorPos,
    track_lines: bool,
    bottom_lines_len: usize,
}

type MaybeError<T = ()> = Result<T, GameError>;

impl GameTui {

    pub fn new() -> Self {
        Self {
            stdout: stdout().into_raw_mode().unwrap(),
            cursor_pos: CursorPos::new(),
            track_lines: false,
            bottom_lines_len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.cursor_pos = CursorPos::new();
    }


    pub fn flush(&mut self) -> MaybeError {
        self.stdout.flush()?;
        Ok(())
    }

    pub fn reset_screen(&mut self) -> MaybeError {
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

    pub fn display_a_line(&mut self, text: &[Text]) -> MaybeError {
        self.display_a_line_raw(text)?;
        self.flush()?;

        Ok(())
    }

    fn display_a_line_raw<T, U>(&mut self, text: U) -> MaybeError
        where
            U: AsRef<[T]>,
        [T]: HasLength,
        T: Display,
        {
            let len = text.as_ref().length() as u16;
            write!(self.stdout, "{}", cursor::Left(len / 2),)?;

            // TODO: find a better way to enable this only in certain contexts
            if self.track_lines {
                let (x, y) = self.stdout.cursor_pos()?;
                self.cursor_pos.lines.push(LinePos { x, y, length: len });
            }

            for t in text.as_ref() {
                self.display_raw_text(t)?;
            }
            write!(self.stdout, "{}", cursor::Left(len),)?;

            Ok(())
        }

    pub fn display_lines<T, U>(&mut self, lines: &[T]) -> MaybeError
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
            self.display_a_line_raw(line.as_ref())?;
        }
        self.flush()?;

        Ok(())
    }

    pub fn display_lines_bottom<T, U>(&mut self, lines: &[T]) -> MaybeError
        where
        T: AsRef<[U]>,
    [U]: HasLength,
    U: Display,
    {
        let (sizex, sizey) = terminal_size()?;

        let line_offset = lines.len() as u16;
        self.bottom_lines_len = lines.len();

        for (line_no, line) in lines.iter().enumerate() {
            write!(
                self.stdout,
                "{}",
                cursor::Goto(sizex / 2, sizey - 1 + (line_no as u16) - line_offset)
                )?;
            self.display_a_line_raw(line.as_ref())?;
        }
        self.flush()?;

        Ok(())
    }

    pub fn display_words(&mut self, words: &[String]) -> MaybeError<Vec<Text>> {
        self.reset();
        let mut current_len = 0;
        let mut max_word_len = 0;
        let mut line = Vec::new();
        let mut lines = Vec::new();
        let (terminal_width, terminal_height) = terminal_size()?;
        // 40% of terminal width
        let max_width = terminal_width * 2 / 5;
        const MAX_WORDS_PER_LINE: usize = 10;
        for word in words {
            max_word_len = std::cmp::max(max_word_len, word.len() + 1);

            let new_len = current_len + word.len() as u16 + 1;
            if line.len() < MAX_WORDS_PER_LINE && new_len <= max_width {
                // add to line
                line.push(word.clone());
                current_len += word.len() as u16 + 1
            } else {
                // add an extra space at the end of each line because
                //  user will instinctively type a space after every word
                //  (at least I did)
                lines.push(Text::from(line.join(" ") + " ").with_faint());

                // clear line
                line = vec![word.clone()];
                current_len = word.len() as u16 + 1;
            }
        }

        lines.push(Text::from(line.join(" ")).with_faint());

        max_word_len = std::cmp::max(max_word_len + 1, MIN_LINE_WIDTH);
        if lines.len() + self.bottom_lines_len + 2 > terminal_height as usize {
            return Err(GameError::from(format!(
                        "Terminal height is too short! Game requires at least {} lines, got {} lines",
                        lines.len() + self.bottom_lines_len + 2,
                        terminal_height,
                        )));
        } else if max_word_len > terminal_width as usize {
            return Err(GameError::from(format!(
                        "Terminal width is too low! Game requires at least {} columns, got {} columns",
                        max_word_len, terminal_width,
                        )));
        }

        self.track_lines = true;
        self.display_lines(
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


    pub fn display_raw_text<T>(&mut self, text: &T) -> MaybeError
        where
        T: Display,
        {
            write!(self.stdout, "{}", text)?;
            Ok(())
        }


    pub fn hide_cursor(&mut self) -> MaybeError {
        write!(self.stdout, "{}", cursor::Hide)?;
        self.flush()?;
        Ok(())
    }


    pub fn show_cursor(&mut self) -> MaybeError {
        write!(self.stdout, "{}", cursor::Show)?;
        self.flush()?;
        Ok(())
    }

    pub fn replace_text<T>(&mut self, text: T) -> MaybeError
        where
        T: Display,
        {
            self.move_to_prev_char()?;
            self.display_raw_text(&text)?;
            self.move_to_cur_pos()?;

            Ok(())
        }


    pub fn move_to_next_char(&mut self) -> MaybeError {
        let (x, y) = self.cursor_pos.next();
        write!(self.stdout, "{}", cursor::Goto(x, y))?;

        Ok(())
    }


    pub fn move_to_prev_char(&mut self) -> MaybeError {
        let (x, y) = self.cursor_pos.prev();
        write!(self.stdout, "{}", cursor::Goto(x, y))?;

        Ok(())
    }


    pub fn move_to_cur_pos(&mut self) -> MaybeError {
        let (x, y) = self.cursor_pos.cur_pos();
        write!(self.stdout, "{}", cursor::Goto(x, y))?;

        Ok(())
    }


    pub fn current_line(&self) -> usize {
        self.cursor_pos.cur_line
    }
}

impl Default for GameTui {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GameTui {

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
