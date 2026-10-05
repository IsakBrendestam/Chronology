//! # Game Client
//!
//! A TCP-based client that has the main purpose
//! of providing the server with information about
//! the state of the game. The client also fetches
//! information about the state of the game from
//! the server, in order to correctly visualize
//! the game.

pub mod client;

pub use client::GameClient;
