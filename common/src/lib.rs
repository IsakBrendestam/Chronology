//! # Game Common
//!
//! Crate with all code that will be shared
//! between the server and client.
//!
//! ## Modules
//! * `chronology_game` - All chronology
//!   game related modules.
//!
//!     * `console_utilities` - Utility
//!       functions, used for handling of
//!       console behaviour.
//!
//!     * `game` - Game logic related
//!       to the chronology game.
//!
//! * `message_definitions` - Functions
//!   and data structures, used for
//!   communication between server and client.

mod chronology_game;
pub use Chronology::ChronologyGame;
pub use Chronology::ChronologyGameError;
pub use chronology_game::game as Chronology;

mod message_definitions;
pub use message_definitions::*;
