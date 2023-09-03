use std::{
    fmt::Display,
    io::{stdout, Stdout, Write},
};

use termion::{
    clear,
    color::{self, Color},
    cursor::{self, DetectCursorPos},
    style, terminal_size,
};

use crate::GameError;

const MIN_LINE_WIDTH: usize = 50;

pub trait HasLength {
    fn length(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct Text {
    raw_text: String,
    formatted_text: String,
    length: usize,
}

impl Text {
    pub fn new(text: String) -> Self {
        let len = text.len();
        Text {
            raw_text: text.clone(),
            formatted_text: text.clone(),
            length: len,
        }
    }

    pub fn raw_text(&self) -> &String {
        &self.raw_text
    }

    pub fn text(&self) -> &String {
        &self.formatted_text
    }

    pub fn with_faint(mut self) -> Self {
        self.raw_text = format!("{}{}{}", style::Faint, self.raw_text, style::NoFaint);
        self
    }

    pub fn with_underline(mut self) -> Self {
        self.raw_text = format!("{}{}{}", style::Underline, self.raw_text, style::Reset);
        self
    }

    pub fn with_color<C>(mut self, color: C) -> Self
        where
        C: Color,
        {
            self.raw_text = format!(
                "{}{}{}",
                color::Fg(color),
                self.raw_text,
                color::Fg(color::Reset)
                );
            self
        }
}

impl HasLength for Text {
    fn length(&self) -> usize {
        self.length
    }
}

impl HasLength for [Text] {
    fn length(&self) -> usize {
        self.iter().map(|t| t.length()).sum()
    }

}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Self::new(text.to_string())
    }
}

impl From<char> for Text {
    fn from(c: char) -> Self {
        Self::new(c.to_string())
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_text)
    }
}


