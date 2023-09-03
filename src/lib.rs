pub mod results;
pub mod tui;
pub mod text;
pub mod markov;

use std::io::{
    StdinLock,
    BufReader,
    BufRead,
    self,
};
use std::path::Path;
use std::time::Instant;
use std::fs::{
    File,
};

use results::GameResults;
use termion::input::Keys;
use termion::{color, event::Key, input::TermRead};
use tui::{GameTui};
use text::Text;
use crate::markov::{
    generate_text,
    create_cache,
};


pub struct Game {
    tui: GameTui,
    text: Vec<Text>,
    words: Vec<String>,
}


pub struct GameError {
    pub msg: String,
}

impl From<std::io::Error> for GameError {
    fn from(error: std::io::Error) -> Self {
        GameError {
            msg: error.to_string(),
        }
    }
}

impl From<String> for GameError {
    fn from(error: String) -> Self {
        GameError { msg: error }
    }
}

impl std::fmt::Debug for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("GameError: {}", self.msg).as_str())
    }
}

impl<'a> Game {
    pub fn new() -> Result<Self, GameError> {

        let mut game = Game {
            tui: GameTui::new(),
            words: Vec::new(),
            text: Vec::new(),
        };

        game.restart()?;

        Ok(game)
    }

    pub fn restart(&mut self) -> Result<(), GameError> {
        self.tui.reset_screen()?;

        let tokens = include_str!("./input.txt")
            .split_whitespace()
            .map(String::from)
            .collect();

        let cache = create_cache(tokens);
        self.words = generate_text(cache, 30);

        self.tui.display_lines_bottom(&[&[
            Text::from("ctrl-r").with_color(color::Blue),
            Text::from(" to restart, ").with_faint(),
            Text::from("ctrl-c").with_color(color::Blue),
            Text::from(" to abort").with_faint(),
        ]])?;

        self.show_words()?;

        Ok(())
    }

    fn show_words(&mut self) -> Result<(), GameError> {
        self.text = self.tui.display_words(&self.words)?;
        Ok(())
    }

    pub fn run(&mut self, stdin: StdinLock<'a>) -> Result<(bool, GameResults), GameError> {
        let mut input = Vec::<char>::new();
        let original_text = self
            .text
            .iter()
            .fold(Vec::<char>::with_capacity(1000), |mut chars, text| {
                chars.extend(text.text().chars());
                chars
            });
        let original_text = self.text.iter()
            .flat_map(|text| text.text().chars())
            .collect::<Vec<_>>();
        let mut num_errors = 0;
        let mut num_chars_typed = 0;

        enum TestStatus {
            // last key press did not quit/restart - more keys to be entered
            NotDone,
            // last letter was typed
            Done,
            // user wants to quit test
            Quit,
            // user wants to restart test
            Restart,
        }

        impl TestStatus {
            fn to_process_more_keys(&self) -> bool {
                matches!(self, TestStatus::NotDone)
            }

            fn to_display_results(&self) -> bool {
                matches!(self, TestStatus::Done)
            }

            fn to_restart(&self) -> bool {
                matches!(self, TestStatus::Restart)
            }
        }

        let mut process_key = |key: Key| -> Result<TestStatus, GameError> {
            match key {
                Key::Ctrl('c') => {
                    return Ok(TestStatus::Quit);
                }
                Key::Ctrl('r') => {
                    return Ok(TestStatus::Restart);
                }
                Key::Ctrl('w') => {
                    // delete last word
                    while !matches!(input.last(), Some(' ') | None) {
                        if input.pop().is_some() {
                            self.tui.replace_text(
                                Text::from(original_text[input.len()]).with_faint(),
                            )?;
                        }
                    }
                }
                Key::Char(c) => {
                    input.push(c);

                    if input.len() >= original_text.len() {
                        return Ok(TestStatus::Done);
                    }

                    num_chars_typed += 1;

                    if original_text[input.len() - 1] == c {
                        self.tui
                            .display_raw_text(&Text::from(c).with_color(color::LightGreen))?;
                        self.tui.move_to_next_char()?;
                    } else {
                        self.tui.display_raw_text(
                            &Text::from(original_text[input.len() - 1])
                                .with_underline()
                                .with_color(color::Red),
                        )?;
                        self.tui.move_to_next_char()?;
                        num_errors += 1;
                    }
                }
                Key::Backspace => {
                    if input.pop().is_some() {
                        self.tui
                            .replace_text(Text::from(original_text[input.len()]).with_faint())?;
                    }
                }
                _ => {}
            }

            self.tui.flush()?;

            Ok(TestStatus::NotDone)
        };

        let mut keys = stdin.keys();

        // read first key
        let key = keys.next().unwrap()?;
        // start the timer
        let started_at = Instant::now();
        // process first key
        let mut status = process_key(key)?;

        if status.to_process_more_keys() {
            for key in &mut keys {
                status = process_key(key?)?;
                if !status.to_process_more_keys() {
                    break;
                }
            }
        }

        // stop the timer
        let ended_at = Instant::now();

        let (final_chars_typed_correctly, final_uncorrected_errors) =
            input.iter().zip(original_text.iter()).fold(
                (0, 0),
                |(total_chars_typed_correctly, total_uncorrected_errors),
                 (typed_char, orig_char)| {
                    if typed_char == orig_char {
                        (total_chars_typed_correctly + 1, total_uncorrected_errors)
                    } else {
                        (total_chars_typed_correctly, total_uncorrected_errors + 1)
                    }
                },
            );

        let results = GameResults {
            total_words: self.words.len(),
            total_chars_typed: num_chars_typed,
            total_chars_in_text: input.len(),
            total_char_errors: num_errors,
            final_chars_typed_correctly,
            final_uncorrected_errors,
            started_at,
            ended_at,
        };

        let to_restart = if status.to_display_results() {
            self.display_results(results.clone(), keys)?
        } else {
            status.to_restart()
        };

        Ok((to_restart, results))
    }

    fn display_results(
        &mut self,
        results: GameResults,
        mut keys: Keys<StdinLock>,
    ) -> Result<bool, GameError> {
        self.tui.reset_screen()?;

        self.tui.display_lines::<&[Text], _>(&[
            &[Text::from(format!(
                "Took {}s for {} words",
                results.duration().as_secs(),
                results.total_words,
            ))],
            &[
                Text::from(format!("Accuracy: {:.1}%", results.accuracy() * 100.0))
                    .with_color(color::Blue),
            ],
            &[Text::from(format!(
                "Mistakes: {} out of {} characters",
                results.total_char_errors, results.total_chars_in_text
            ))],
            &[
                Text::from("Speed: "),
                Text::from(format!("{:.1} wpm", results.wpm())).with_color(color::Green),
                Text::from(" (words per minute)"),
            ],
        ])?;
        self.tui.display_lines_bottom(&[&[
            Text::from("ctrl-r").with_color(color::Blue),
            Text::from(" to restart, ").with_faint(),
            Text::from("ctrl-c").with_color(color::Blue),
            Text::from(" to quit ").with_faint(),
        ]])?;
        // no cursor on results page
        self.tui.hide_cursor()?;

        let mut to_restart: Option<bool> = None;
        while to_restart.is_none() {
            match keys.next().unwrap()? {
                // press ctrl + 'r' to restart
                Key::Ctrl('r') => to_restart = Some(true),
                // press ctrl + 'c' to quit
                Key::Ctrl('c') => to_restart = Some(false),
                _ => {}
            }
        }

        self.tui.show_cursor()?;

        Ok(to_restart.unwrap_or(false))
    }
}
