//! # Game Server
//!
//! A TCP-based server that has the main purpose
//! of handling events related to and keeping
//! track of the starte of a chronology game
//! instance for each client.
//!
//! The server currently support multiple clients,
//! but each client has it's own state and game.
//! Thus there is no support for multiplayer.

pub mod server_implementation;
pub use server_implementation::server::GameServer;
