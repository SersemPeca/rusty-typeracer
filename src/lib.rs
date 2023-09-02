
pub mod text;
pub mod results;
pub mod tui;

use std::io::{
    StdinLock,
    BufReader,
    BufRead,
};
use std::path::{
    PathBuf,
    Path,
};
use std::fs::File;
use std::time::Instant;
use termion::event::Key;
use termion::input::{
    TermRead,
    Keys,
};
use termion::color;

use crate::tui::GameTui;
use crate::text::Text;
use crate::results::GameResults;

pub struct GameError {
    pub msg: String,
}

impl From<String> for GameError {
    fn from(err: String) -> Self {
        GameError {
            msg: err,
        }
    }
}

impl From<std::io::Error> for GameError {
    fn from(err: std::io::Error) -> Self {
        GameError {
            msg: err.to_string(),
        }
    }
}

impl std::fmt::Debug for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("GameError: {}", self.msg).as_str())
    }
}

pub struct Game {
    tui: GameTui,
    text: Vec<Text>,
    words: Vec<String>,
}

impl<'a> Game {
    fn lines_from_file(filename: impl AsRef<Path>) -> Vec<String> {
        let file = File::open(filename).expect("no such file");
        let buf = BufReader::new(file);
        buf.lines()
            .map(|l| l.expect("Could not parse line"))
            .collect()
    }

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

        self.words = Self::lines_from_file("./words");

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
            .fold(Vec::<char>::new(), |mut chars, text| {
                chars.extend(text.text().chars());
                chars
            });
        let mut num_errors = 0;
        let mut num_chars_typed = 0;

        enum GameStatus {
            NotDone,
            Done,
            Quit,
            Restart,
        }

        impl GameStatus {
            fn to_process_more_keys(&self) -> bool {
                matches!(self, GameStatus::NotDone)
            }

            fn to_display_results(&self) -> bool {
                matches!(self, GameStatus::Done)
            }

            fn to_restart(&self) -> bool {
                matches!(self, GameStatus::Restart)
            }
        }

        let mut process_key = |key: Key| -> Result<GameStatus, GameError> {
            match key {
                Key::Ctrl('c') => {
                    return Ok(GameStatus::Quit);
                }
                Key::Ctrl('r') => {
                    return Ok(GameStatus::Restart);
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
                        return Ok(GameStatus::Done);
                    }

                    num_chars_typed += 1;

                    if original_text[input.len() - 1] == c {
                        self.tui
                            .print_text_raw(&Text::from(c).with_color(color::LightGreen))?;
                        self.tui.move_to_next_char()?;
                    } else {
                        self.tui.print_text_raw(
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

            Ok(GameStatus::NotDone)
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

        self.tui.print_lines::<&[Text], _>(&[
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
        // no cursor on results page
        self.tui.hide_cursor()?;

        // TODO: make this a bit more general
        // perhaps use a `known_keys_pressed` flag?
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
