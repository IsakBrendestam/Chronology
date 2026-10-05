//! # Chronology Game
//!
//! This module contains logic for running and
//! managing the chronology game.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use rand::{Rng, rng};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum GamePhase {
    Running,
    Waiting,
}

/// Error type representing different error
/// possibilities for the game.
#[derive(Debug)]
pub enum ChronologyGameError {
    NoEvents,
    FailedLoadingDatabase(std::io::Error),
    FailedParsingData(serde_json::Error),
}

impl PartialEq for ChronologyGameError {
    fn eq(&self, other: &Self) -> bool {
        use ChronologyGameError::*;
        match (self, other) {
            (NoEvents, NoEvents) => true,
            (FailedLoadingDatabase(e1), FailedLoadingDatabase(e2)) => e1.kind() == e2.kind(),
            (FailedParsingData(e1), FailedParsingData(e2)) => e1.classify() == e2.classify(),
            _ => false,
        }
    }
}

impl std::fmt::Display for ChronologyGameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChronologyGameError::NoEvents => write!(f, "Game does not contain any events."),
            ChronologyGameError::FailedLoadingDatabase(e) => {
                write!(f, "Failed to load database: {}", e)
            }
            ChronologyGameError::FailedParsingData(e) => write!(f, "Failed to parse data: {}", e),
        }
    }
}

pub type YearType = i16;
pub type GuessType = i8;

/// Data representing an Event. That's used for
/// displaying and evaluating guesses in the game.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct Event {
    pub category: String,
    pub year: YearType,
    pub text: String,
}

/// Wraps the [`Event`] struct with additional
/// information useful for the game.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct GameEventWrapper {
    pub event: Event,
    pub frozen: bool,
}

/// Data storing the state of the game.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChronologyGame {
    active_events: Vec<GameEventWrapper>,

    #[serde(skip)]
    events: Vec<Event>,

    drawn_events: HashSet<Event>,
    round: u8,
    active_to_win: u8,
    last_added_index: i8,
    data_file_path: String,
}

impl PartialEq for ChronologyGame {
    // Igore events
    fn eq(&self, other: &Self) -> bool {
        self.active_events == other.active_events
            && self.round == other.round
            && self.active_to_win == other.active_to_win
            && self.last_added_index == other.last_added_index
            && self.data_file_path == other.data_file_path
    }
}

impl Default for ChronologyGame {
    fn default() -> Self {
        Self::new()
    }
}

impl ChronologyGame {
    /// Creates a new `ChronologyGame` whith
    /// default values.
    pub fn new() -> Self {
        Self {
            active_events: Vec::new(),
            events: Vec::new(),
            drawn_events: HashSet::new(),
            round: 1,
            active_to_win: 10,
            last_added_index: -1,
            data_file_path: String::from("../d7082e_db/events.json"),
        }
    }

    /// Loads data from the database. All data will
    /// be stored in `d7082e_db/events.json`, if the
    /// file does not exists, the developer probably
    /// have to run `cargo test` in the d7082e_db
    /// crate.
    ///
    /// # Returns
    ///
    /// If successfull [`Ok`] will be returned.
    /// Otherwise [`Err<ChronologyGameError>`] will
    /// be returned signifying what went wrong.
    pub fn load(&mut self) -> Result<(), ChronologyGameError> {
        // Local file path
        let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&self.data_file_path);

        let data =
            fs::read_to_string(file_path).map_err(ChronologyGameError::FailedLoadingDatabase)?;

        self.events =
            serde_json::from_str(&data).map_err(ChronologyGameError::FailedParsingData)?;

        self.add_initial_event()
    }

    /// Restarts the chronology game, and stes the
    /// state to what is was initially.
    ///
    /// # Returns
    ///
    /// If successfull [`Ok`] will be returned.
    /// Otherwise [`Err<ChronologyGameError>`] will
    /// be returned signifying what went wrong.
    pub fn restart(&mut self) -> Result<(), ChronologyGameError> {
        self.drawn_events.clear();
        self.active_events.clear();
        self.round = 1;

        self.reset_last_correct_index();
        self.add_initial_event()
    }

    /// Adds the initial event to the timeline.
    /// Note: that the event will be frozen.
    fn add_initial_event(&mut self) -> Result<(), ChronologyGameError> {
        let event = self.generate_event()?;
        self.add_event(&event, 0, true);
        Ok(())
    }

    /// Prints the victory information to [`std::io::stdout`].
    pub fn display_victory_screen(&self) {
        println!("------------------------------");
        println!("########## You Won! ##########");
        println!("------------------------------");
        println!("########## Score: {} ##########", self.round);
        println!("------------------------------");
    }

    /// Evaluates a guess, and if the guess is
    /// correct it will be registerd and added
    /// to the timeline.
    ///
    /// # Arguments
    ///
    /// * `event` - The event related to the
    ///   guess, used for verifying correctness.
    ///
    /// * `guess` - Index relative the timeline,
    ///   representing the user guess.
    pub fn add_guess(&mut self, event: Event, guess: GuessType) -> bool {
        if self.evaluate_guess(&event, guess) {
            self.add_event(&event, guess as usize, false);
            return true;
        }
        false
    }

    /// Returns `true` if the last provided guess
    /// was correct, otherwise `false` will be
    /// returned.
    pub fn was_last_correct(&self) -> bool {
        self.last_added_index != -1
    }

    /// Resets the index that reprecents the latest
    /// event that was added to the timeline.
    pub fn reset_last_correct_index(&mut self) {
        self.last_added_index = -1;
    }

    /// Returns the last correct event, if the last
    /// guess was correct.
    pub fn last_correct_event(&self) -> Option<Event> {
        if self.last_added_index == -1 {
            return None;
        }
        Some(
            self.active_events[self.last_added_index as usize]
                .event
                .clone(),
        )
    }

    /// Generates a random event from the database.
    /// It will return an error if there are no
    /// events left.
    pub fn generate_event(&mut self) -> Result<Event, ChronologyGameError> {
        let valid_events: Vec<&Event> = self
            .events
            .iter()
            .filter(|event| !self.drawn_events.contains(event))
            .collect();

        if !valid_events.is_empty() {
            let random_index = rng().random_range(0..=valid_events.len() - 1);
            let event = valid_events[random_index].clone();
            self.add_drawn(&event);
            return Ok(event);
        }
        Err(ChronologyGameError::NoEvents)
    }

    /// Mark a specific event as drawn.
    fn add_drawn(&mut self, event: &Event) {
        self.drawn_events.insert(event.clone());
    }

    /// Evaluates a provied guess against a provied event.
    /// If the guess generates years that encloses the year
    /// of the provied event true will be returned. Otherwise
    /// false will be returned.
    pub fn evaluate_guess(&self, event: &Event, guess: GuessType) -> bool {
        let min_index = (guess - 1).min((self.active_events.len() - 1) as i8) as usize;
        let min = self
            .active_events
            .get(min_index)
            .map(|y| y.event.year)
            .unwrap_or(YearType::MIN);

        let max_index = guess.max(0) as usize;
        let max = self
            .active_events
            .get(max_index)
            .map(|y| y.event.year)
            .unwrap_or(YearType::MAX);

        event.year >= min && event.year <= max
    }

    /// Adds an event to the board at a specified index.
    /// The index will be clamped to a valid index and
    /// the index representing the last added event
    /// will be updated.
    ///
    /// # Arguments
    ///
    /// * `event` - event that is to be added
    ///   to the board
    ///
    /// * `index` - index where the event will be inserted
    ///   to the board.
    ///
    /// * `frozen` - if true the added event will be
    ///   registerd as frozen, otherwise it will not.
    fn add_event(&mut self, event: &Event, index: usize, frozen: bool) {
        let valid_index = index.min(self.active_events.len());
        self.active_events.insert(
            valid_index,
            GameEventWrapper {
                event: event.clone(),
                frozen,
            },
        );
        self.last_added_index = valid_index as i8;
    }

    /// prints the board to [`std::io::stdout`] with
    /// propper formatting and text coloring.
    ///
    /// # Arguments
    ///
    /// * `correct_guess` - if true the input will
    ///   be displayed as if the last added event was
    ///   correctly guessed by the user, otherwise
    ///   the board will be visualized with the
    ///   non-frozen events that will be lost highlighted.
    pub fn display_board(&self, correct_guess: bool) {
        clear!();

        println!("{}\n", "~".repeat((self.active_events.len() + 1) * 8));

        if !correct_guess {
            set_red!();
        }

        self.display_years(false);
        self.display_connectors(false);

        if !correct_guess {
            reset_color!();
        }

        self.display_timeline();

        set_blue!();

        self.display_connectors(true);
        self.display_years(true);

        reset_color!();

        println!("\n{}", "~".repeat((self.active_events.len() + 1) * 8));
    }

    /// Displays the years of all events.
    ///
    /// # Arguments
    ///
    /// * `print_frozen` - boolean providing
    ///   option wheter to display forzen or
    ///   non-frozen events.
    fn display_years(&self, print_frozen: bool) {
        print!("   ");
        for (index, event) in self.active_events.iter().enumerate() {
            let print_latest = (index as i8) == self.last_added_index && !print_frozen;

            if print_latest {
                set_green!();
            }

            match event.frozen == print_frozen {
                true => print!("{:>8}", event.event.year),
                false => print!("{:>8}", ""),
            }

            if print_latest {
                reset_color!();
            }
        }
        println!();
    }

    /// Displays connectors from the yars to
    /// the timelin.
    ///
    /// # Arguments
    ///
    /// * `print_frozen` - boolean providing
    ///   option wheter to display forzen or
    ///   non-frozen events.
    fn display_connectors(&self, print_frozen: bool) {
        print!("   ");
        for (index, year) in self.active_events.iter().enumerate() {
            let print_latest = (index as i8) == self.last_added_index && !print_frozen;

            if print_latest {
                set_green!();
            }

            match year.frozen == print_frozen {
                true => print!("{:>7} ", "|"),
                false => print!("{:>7} ", ""),
            }

            if print_latest {
                reset_color!();
            }
        }
        println!();
    }

    /// Displays the timelin with option values.
    fn display_timeline(&self) {
        print!("\x1b[33m{0:>2}\x1b[0m --0", self.round);
        for index in 1..self.active_events.len() + 1 {
            let index_digits = index.checked_ilog10().unwrap_or(0);
            print!("{0}{1}", "-".repeat((7 - index_digits) as usize), index);
        }
        println!("--");
    }

    /// Returns boolean values representing whether
    /// the conditions for winning the game has been
    /// fulfilled.
    pub fn check_done(&self) -> bool {
        self.active_events.len() >= self.active_to_win as usize
    }

    /// Frezes all the events on the board.
    pub fn freeze_board(&mut self) {
        self.round += 1;
        for event in self.active_events.iter_mut() {
            event.frozen = true;
        }
    }

    /// Prints information presenting for the user
    /// that the guess for `event` was incorrect.
    pub fn incorrect_guess(&mut self) {
        self.round += 1;
        self.active_events.retain(|y| y.frozen);
        self.last_added_index = -1;
    }

    /// Promts the user for input wheter to
    /// continue playing the game or not.
    ///
    /// # Returns
    ///
    /// `true` will be returned if the answare
    /// is yes, otherwise `false` will be returned.
    pub async fn user_restart_option(&self) -> bool {
        print!("\nDo you want to play again (y/n): ");
        flush!();

        let input = smol::unblock(|| {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            Ok::<String, std::io::Error>(line)
        })
        .await;

        if let Ok(input) = input {
            return matches!(input.trim().to_lowercase().as_str(), "y" | "yes");
        }
        false
    }

    /// Returns the round/score
    pub fn get_round(&self) -> u8 {
        self.round
    }

    /// Returns the first active event
    pub fn get_first_event(&self) -> Event {
        self.active_events[0].event.clone()
    }

    pub fn get_active_events(&self) -> Vec<GameEventWrapper> {
        self.active_events.clone()
    }
}

#[cfg(test)]
mod chronology_game_tests {
    use super::*;

    #[test]
    fn create_game_test() {
        let mut game = ChronologyGame::default();
        game.load().expect("Failed to load game.");

        assert_eq!(game.active_events.len(), 1);
        assert_eq!(game.round, 1);
        assert!(!game.events.is_empty());
    }

    #[test]
    fn restart_game_test() {
        let mut game = ChronologyGame::default();
        game.load().expect("Failed to load game.");

        let event = game.generate_event();
        game.add_event(&event.unwrap(), 1, false);

        let event = game.generate_event();
        game.add_event(&event.unwrap(), 2, false);

        let event = game.generate_event();
        game.add_event(&event.unwrap(), 2, true);

        let event = game.generate_event();
        game.add_event(&event.unwrap(), 0, false);

        let event = game.generate_event();
        game.add_event(&event.unwrap(), 4, true);

        game.restart().expect("Failed to restat game.");

        assert!(!game.events.is_empty());
        assert_eq!(game.active_events.len(), 1);
        assert_eq!(game.round, 1);
    }

    #[test]
    fn handle_empty_events_test() {
        let mut game = ChronologyGame::default();
        game.load().expect("Failed to load game.");

        game.events.clear();

        let event = game.generate_event();
        assert!(event.is_err());
    }

    #[test]
    fn drain_events_test() {
        let mut game = ChronologyGame::default();
        game.load().expect("Failed to load game.");

        let n_events = game.events.len();
        for _ in 0..n_events + 50 {
            let _ = game.generate_event();
        }
    }

    #[test]
    fn guess_evaluation_test() {
        let mut game = ChronologyGame::default();
        game.load().expect("Failed to load game.");
        game.active_events.clear();

        // Guess: 0
        game.active_events.push(GameEventWrapper {
            event: Event {
                category: String::new(),
                year: 1990,
                text: String::new(),
            },
            frozen: true,
        });
        // Guess: 1
        game.active_events.push(GameEventWrapper {
            event: Event {
                category: String::new(),
                year: 2000,
                text: String::new(),
            },
            frozen: true,
        });
        // Guess: 2
        game.active_events.push(GameEventWrapper {
            event: Event {
                category: String::new(),
                year: 2010,
                text: String::new(),
            },
            frozen: true,
        });
        // Guess: 3

        let mut temp_event = Event {
            category: String::new(),
            year: 0,
            text: String::new(),
        };

        temp_event.year = 1888;
        assert!(game.evaluate_guess(&temp_event, -1));
        assert!(game.evaluate_guess(&temp_event, 0));
        assert!(!game.evaluate_guess(&temp_event, 1));
        assert!(!game.evaluate_guess(&temp_event, 2));
        assert!(!game.evaluate_guess(&temp_event, 3));
        assert!(!game.evaluate_guess(&temp_event, 4));

        temp_event.year = 1995;
        assert!(!game.evaluate_guess(&temp_event, -1));
        assert!(!game.evaluate_guess(&temp_event, 0));
        assert!(game.evaluate_guess(&temp_event, 1));
        assert!(!game.evaluate_guess(&temp_event, 2));
        assert!(!game.evaluate_guess(&temp_event, 3));
        assert!(!game.evaluate_guess(&temp_event, 4));

        temp_event.year = 2003;
        assert!(!game.evaluate_guess(&temp_event, -1));
        assert!(!game.evaluate_guess(&temp_event, 0));
        assert!(!game.evaluate_guess(&temp_event, 1));
        assert!(game.evaluate_guess(&temp_event, 2));
        assert!(!game.evaluate_guess(&temp_event, 3));
        assert!(!game.evaluate_guess(&temp_event, 4));

        temp_event.year = 2143;
        assert!(!game.evaluate_guess(&temp_event, -1));
        assert!(!game.evaluate_guess(&temp_event, 0));
        assert!(!game.evaluate_guess(&temp_event, 1));
        assert!(!game.evaluate_guess(&temp_event, 2));
        assert!(game.evaluate_guess(&temp_event, 3));
        assert!(game.evaluate_guess(&temp_event, 4));
    }

    #[test]
    fn check_win_test() {
        let mut game = ChronologyGame::default();
        game.load().expect("Failed to load game.");

        while game.active_events.len() < game.active_to_win as usize {
            let temp_event = game.generate_event().expect("Faiedl to generate event.");
            let mut temp_index = game.active_events.len();
            for (index, event) in game.active_events.iter().enumerate() {
                if temp_event.year <= event.event.year {
                    temp_index = index;
                    break;
                }
            }
            game.add_event(&temp_event, temp_index, false);
        }

        assert!(game.check_done());
    }
}
