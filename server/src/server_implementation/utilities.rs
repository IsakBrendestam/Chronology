//! # Server Utilities
//!
//! A collection of uility functions and data-
//! structures used by the server.

use std::{collections::HashMap, sync::Arc};

use smol::lock::Mutex;
use smol::net::TcpStream;

use game_common::*;

#[derive(Debug)]
pub enum ServerError {
    UnexpectedDisconnect,
    MessageError(smol::io::Error),
    InvalidMessage,
    InvalidCookie,
    GameError(ChronologyGameError),
    InvalidReconnect,
    InvalidClientPhase,
    NotCurrentPlayer,
    NoActivePlayers,
}

impl PartialEq for ServerError {
    fn eq(&self, other: &Self) -> bool {
        use ServerError::*;

        match (self, other) {
            (UnexpectedDisconnect, UnexpectedDisconnect) => true,
            (MessageError(e1), MessageError(e2)) => e1.kind() == e2.kind(),
            (InvalidMessage, InvalidMessage) => true,
            (InvalidCookie, InvalidCookie) => true,
            (GameError(_), GameError(_)) => true,
            (InvalidReconnect, InvalidReconnect) => true,
            (InvalidClientPhase, InvalidClientPhase) => true,
            (NotCurrentPlayer, NotCurrentPlayer) => true,
            (NoActivePlayers, NoActivePlayers) => true,

            _ => false,
        }
    }
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::MessageError(e) => write!(f, "Could not send message: {}", e),
            ServerError::UnexpectedDisconnect => write!(f, "Client unexpectedly disconnected."),
            ServerError::InvalidMessage => write!(
                f,
                "Server received and invaild message for its current phase."
            ),
            ServerError::InvalidCookie => write!(f, "Invalid Cookie."),
            ServerError::GameError(e) => write!(f, "Failed loading game: {}", e),
            ServerError::InvalidReconnect => write!(f, "The reconnection attempt was invalid."),
            ServerError::InvalidClientPhase => write!(f, "Invalid client phase,"),
            ServerError::NotCurrentPlayer => write!(f, "Trying to play out of turn."),
            ServerError::NoActivePlayers => write!(f, "All player clients are disconnected."),
        }
    }
}

impl From<smol::io::Error> for ServerError {
    fn from(e: smol::io::Error) -> Self {
        if e.kind() == smol::io::ErrorKind::UnexpectedEof {
            ServerError::UnexpectedDisconnect
        } else {
            ServerError::MessageError(e)
        }
    }
}

impl From<ChronologyGameError> for ServerError {
    fn from(e: ChronologyGameError) -> Self {
        ServerError::GameError(e)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerPhase {
    Join,
    Run,
}

/// Stores game state for each client, identified
/// by cookies, randomy generated when connecting.
pub struct ServerState {
    pub(super) clients: HashMap<Cookie, GameState>,
    pub(super) streams: HashMap<Cookie, TcpStream>,
    pub(super) phase: ServerPhase,

    pub(super) client_order: Vec<Cookie>,
    pub(super) current_player: usize,
    pub(super) session_owner: Option<Cookie>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            streams: HashMap::new(),
            phase: ServerPhase::Join,
            client_order: Vec::new(),
            current_player: 0,
            session_owner: None,
        }
    }
}

pub mod server_utilities {
    use super::*;

    /// Sends provided message to all connected
    /// clients.
    pub async fn send_message_to_all(
        server_state: Arc<Mutex<ServerState>>,
        message: ToClient,
    ) -> Result<(), ServerError> {
        let mut state = server_state.lock().await;
        for stream in state.streams.values_mut() {
            send_message(stream, message.clone()).await?;
        }
        Ok(())
    }

    /// Generates a unique random cookie.
    pub async fn generate_cookie(server_state: Arc<Mutex<ServerState>>) -> Cookie {
        let state = server_state.lock().await;
        loop {
            let cookie: Cookie = rand::random();
            if !state.clients.contains_key(&cookie) {
                return cookie;
            }
        }
    }

    /// Returns the session owner.
    pub async fn get_session_owner(server_state: Arc<Mutex<ServerState>>) -> Option<Cookie> {
        let state = server_state.lock().await;
        state.session_owner
    }

    /// Sends a list of all connected players names
    /// to all connected clients.
    pub async fn send_player_list(
        server_state: Arc<Mutex<ServerState>>,
    ) -> Result<(), ServerError> {
        let mut player_list = Vec::new();
        {
            let state = server_state.lock().await;

            for (_, game_state) in state.clients.iter() {
                player_list.push(game_state.username.clone());
            }
        }
        send_message_to_all(server_state, ToClient::PlayerList(player_list)).await
    }

    /// Checks if the phase of the client with
    /// `cookie` mathces `phase`.
    pub async fn check_client_phase(
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
        phase: Chronology::GamePhase,
    ) -> Result<bool, ServerError> {
        let mut state = server_state.lock().await;
        if let Some(game_state) = state.clients.get_mut(&cookie) {
            return Ok(game_state.phase == phase);
        }
        Ok(false)
    }

    /// Makes the scenario where the player should
    /// rejoin the ongoing game.
    pub async fn handle_player_rejoin(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        send_message(stream, ToClient::RejoinSession).await?;

        if check_current_player(server_state.clone(), cookie).await {
            send_message(stream, ToClient::StartIteration).await?;
        }

        let mut state = server_state.lock().await;
        match state.clients.get_mut(&cookie) {
            Some(game_sate) => {
                game_sate.client_disconnected = false;
                Ok(())
            }
            None => Err(ServerError::InvalidCookie),
        }
    }

    /// Check if provided cookie is related to
    /// the current player.
    pub async fn check_current_player(
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> bool {
        let state = server_state.lock().await;
        state.client_order[state.current_player] == cookie
    }

    /// Sets the next player to a player that
    /// is connected to the game.
    pub async fn set_next_player(
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        if get_active_player_count(server_state.clone()).await == 0 {
            return Err(ServerError::NoActivePlayers);
        }

        let mut state = server_state.lock().await;

        let game_state = state
            .clients
            .get_mut(&cookie)
            .expect("This should not be possible"); // Because of previous checks

        game_state.phase = Chronology::GamePhase::Waiting;
        let current_cookie;
        loop {
            state.current_player = (state.current_player + 1) % state.client_order.len();
            let temp_cookie = state.client_order[state.current_player];

            if let Some(game_state) = state.clients.get(&temp_cookie)
                && !game_state.client_disconnected
            {
                current_cookie = temp_cookie;
                break;
            }
        }

        match state.clients.get_mut(&current_cookie) {
            Some(game_sate) => {
                game_sate.phase = Chronology::GamePhase::Running;
                game_sate.should_wait = false;
                if let Some(current_stream) = state.streams.get_mut(&current_cookie) {
                    send_message(current_stream, ToClient::StartIteration).await?;
                }
                Ok(())
            }
            None => Err(ServerError::InvalidCookie),
        }
    }

    pub async fn get_active_player_count(server_state: Arc<Mutex<ServerState>>) -> usize {
        let state = server_state.lock().await;
        state
            .clients
            .values()
            .filter(|v| !v.client_disconnected)
            .count()
    }

    pub async fn get_game_state<'a>(
        server_state: &'a ServerState,
        cookie: Cookie,
        stream: &mut TcpStream,
    ) -> Result<&'a GameState, ServerError> {
        match server_state.clients.get(&cookie) {
            Some(game_state) => Ok(game_state),
            None => {
                println!(
                    "Server: Invalid cookie from: {:?}",
                    stream.peer_addr().unwrap()
                );
                send_message(stream, ToClient::InvalidCookie).await?;
                Err(ServerError::InvalidCookie)
            }
        }
    }

    pub async fn get_game_state_mut<'a>(
        server_state: &'a mut ServerState,
        cookie: Cookie,
        stream: &mut TcpStream,
    ) -> Result<&'a mut GameState, ServerError> {
        match server_state.clients.get_mut(&cookie) {
            Some(game_state) => Ok(game_state),
            None => {
                println!(
                    "Server: Invalid cookie from: {:?}",
                    stream.peer_addr().unwrap()
                );
                send_message(stream, ToClient::InvalidCookie).await?;
                Err(ServerError::InvalidCookie)
            }
        }
    }
}
