//! # Game Server
//!
//! Handles requests from clients and keeps tack
//! of game state. The server completely owns the
//! state of the game, making sure clients are
//! not able to cheat the game.
//!
//! # Action Scheme:
//! When the server recieves a [`ToServer`] the
//! following message is sent as responce:
//!
//! * [`ToServer::Connect`]             -> [`ToClient::ConnectionAccepted`]
//!
//! * [`ToServer::Disconnect`]          -> None
//!
//! * [`ToServer::Reconnect`]           -> [`ToClient::ReconnectAccepted`]
//!
//! * [`ToServer::StartGame`]           -> None
//!
//! * [`ToServer::RestartGame`]         -> None
//!
//! * [`ToServer::RequestState`]        -> [`ToClient::State`]
//!
//! * [`ToServer::Guess`]               -> None
//!
//! * [`ToServer::IncorrectGuess`]      -> None
//!
//! * [`ToServer::Freeze`]              -> None
//!
//! * [`ToServer::RestartIteration`]    -> None

use super::utilities::{ServerError, ServerPhase, ServerState, server_utilities};

use Chronology::{ChronologyGame, GamePhase};
use game_common::Chronology::YearType;
use game_common::*;

use smol::lock::Mutex;
use smol::net::{TcpListener, TcpStream};

use std::sync::Arc;

/// Stores any infromation related to the server,
/// such as its state, ip and port.
pub struct GameServer {
    ip: String,
    port: String,
    server_state: Arc<Mutex<ServerState>>,
}

impl GameServer {
    /// Creates a new instance of the game server.
    ///
    /// # Arguments
    ///
    /// * `ip` - ip address used by server.
    ///
    /// * `port` - port used by server.
    pub fn new(ip: &str, port: &str) -> Self {
        Self {
            ip: String::from(ip),
            port: String::from(port),
            server_state: Arc::new(Mutex::new(ServerState::new())),
        }
    }

    /// Starts the server, running asynchronously.
    ///
    /// # Arguments
    ///
    /// * `stall` - if true the function will
    ///   await execution of the server. Otherwise
    ///   the server will be detached.
    pub fn start(&mut self, stall: bool) {
        let state = self.server_state.clone();

        let ip = self.ip.clone();
        let port = self.port.clone();

        smol::block_on(async move {
            let server = smol::spawn(async move {
                if let Err(e) = Self::run(ip, port, state).await {
                    eprintln!("Server error: {}", e);
                }
            });

            if stall {
                server.await;
            } else {
                server.detach();
            }
        });
    }

    /// Creates the TCP-listner and handles any
    /// incomming clients.
    ///
    /// # Arguments
    ///
    /// * `ip` - ip address of the server.
    ///
    /// * `port` - port used by server.
    ///
    /// * `server_state` - state of the server.
    async fn run(
        ip: String,
        port: String,
        server_state: Arc<Mutex<ServerState>>,
    ) -> Result<(), ServerError> {
        let tcp_listener = TcpListener::bind(format!("{0}:{1}", ip, port)).await?;
        println!("Starting Server {0}:{1}", ip, port);

        loop {
            let (mut stream, _) = tcp_listener.accept().await?;
            let state = server_state.clone();

            smol::spawn(async move {
                // Connect client
                let connection_result = Self::handle_connection(&mut stream, state.clone()).await;

                let Ok(cookie) = connection_result else {
                    eprintln!(
                        "Failed to connect client: {}",
                        connection_result.unwrap_err()
                    );
                    return;
                };

                // Handle client communation
                let mut invalid_cookie_count: u8 = 0; // Number of times invalid cookie provided
                let mut not_current_player_count: u8 = 0; // Nomber of times acting out of turn
                let mut message_error_count: u8 = 0; // Number of times message errors occured
                loop {
                    if let Err(error) = Self::client_handler(&mut stream, state.clone()).await {
                        match error {
                            // UnexpectedDisconnect handle
                            ServerError::UnexpectedDisconnect => {
                                if let Err(d_error) = Self::disconnect(state.clone(), cookie).await
                                {
                                    eprintln!("Server Error: {}", d_error);
                                }
                                break;
                            }

                            // InvalidCookie handle
                            ServerError::InvalidCookie if invalid_cookie_count > 3 => {
                                if let Err(d_error) = Self::disconnect(state.clone(), cookie).await
                                {
                                    eprintln!("Server Error: {}", d_error);
                                }
                                break;
                            }
                            ServerError::InvalidCookie => invalid_cookie_count += 1,

                            // NotCurrentPlayer handle
                            ServerError::NotCurrentPlayer if not_current_player_count > 3 => {
                                if let Err(d_error) = Self::disconnect(state.clone(), cookie).await
                                {
                                    eprintln!("Server Error: {}", d_error);
                                }
                                break;
                            }
                            ServerError::NotCurrentPlayer => not_current_player_count += 1,

                            // MessageError handle
                            ServerError::MessageError(e) if message_error_count > 3 => {
                                if let Err(d_error) = Self::disconnect(state.clone(), cookie).await
                                {
                                    eprintln!("Server Error: {}", d_error);
                                }
                                eprintln!("Server Error: {}", e);
                                break;
                            }
                            ServerError::MessageError(_) => message_error_count += 1,

                            other_error => eprintln!("Server Error: {}", other_error),
                        }
                    }
                }
                println!("Dropping connection");
            })
            .detach();
        }
    }

    /// Handles connection or reconnection with
    /// incomming player.
    async fn handle_connection(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
    ) -> Result<Cookie, ServerError> {
        loop {
            let message: ToServer = receive_message(stream).await?;

            match message {
                ToServer::Connect(username) => {
                    let cookie = server_utilities::generate_cookie(server_state.clone()).await;
                    Self::connect(cookie, stream, server_state.clone(), username).await?;
                    return Ok(cookie);
                }

                ToServer::Reconnect(cookie) => {
                    if let Err(e) = Self::reconnect(stream, server_state.clone(), cookie).await {
                        if e != ServerError::InvalidCookie {
                            return Err(e);
                        }
                    } else {
                        return Ok(cookie);
                    }
                }

                invalid => println!(
                    "Message discurraged, invalid message during connection phase. Message: {:?}",
                    invalid
                ),
            }
        }
    }

    /// Establishes a new connection with a client.
    /// By generating a cookie and sending
    /// confiromation message to client. A game
    /// instance is also generated.
    async fn connect(
        generated_cookie: Cookie,
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        username: String,
    ) -> Result<(), ServerError> {
        let mut game = ChronologyGame::new();
        game.load()?;

        // Generate unique cookie
        {
            let mut state = server_state.lock().await;
            state.clients.insert(
                generated_cookie,
                GameState {
                    game: game.clone(),
                    current_event: None,
                    incorrect_event: None,
                    username,
                    should_wait: true,
                    phase: Chronology::GamePhase::Waiting,
                    client_disconnected: false,
                },
            );
        }

        send_message(stream, ToClient::ConnectionAccepted(generated_cookie)).await?;
        println!("Server: Connection: {:?} ", stream.peer_addr().unwrap());

        let phase;
        {
            let mut state = server_state.lock().await;

            state.streams.insert(generated_cookie, stream.clone());
            phase = state.phase.clone();

            if state.session_owner.is_none() {
                state.session_owner = Some(generated_cookie);
                send_message(stream, ToClient::SessionOwner).await?;
            }
        }

        if phase == ServerPhase::Join {
            server_utilities::send_player_list(server_state).await?;
        }

        Ok(())
    }

    /// Handles any message from clients from
    /// client and performes appropriate action.
    async fn client_handler(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
    ) -> Result<(), ServerError> {
        loop {
            let phase;
            {
                let state = server_state.lock().await;
                phase = state.phase.clone();
            }
            println!("Phase: {:?}", phase);

            match phase {
                ServerPhase::Join => Self::join_phase(stream, server_state.clone()).await?,
                ServerPhase::Run => Self::run_phase(stream, server_state.clone()).await?,
            }
        }
    }

    /// Handles communication with client during
    /// the join phase.
    async fn join_phase(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
    ) -> Result<(), ServerError> {
        loop {
            let message: ToServer = receive_message(stream).await?;

            match message {
                ToServer::StartSession(cookie)
                    if Some(cookie)
                        == server_utilities::get_session_owner(server_state.clone()).await =>
                {
                    server_utilities::send_message_to_all(
                        server_state.clone(),
                        ToClient::StartSession,
                    )
                    .await?;
                    Self::start_session(server_state.clone()).await?;
                }

                ToServer::StartGame(cookie) => {
                    Self::request_state(stream, server_state.clone(), cookie).await?;
                    Self::start_game(stream, server_state.clone(), cookie).await?;
                    break;
                }

                invalid => println!(
                    "Message discurraged, invalid message for server join phase. Message: {:?}",
                    invalid
                ),
            }
        }

        Ok(())
    }

    /// Starts a new game session with all connected
    /// players. Also updates the server phase.
    async fn start_session(server_state: Arc<Mutex<ServerState>>) -> Result<(), ServerError> {
        let mut state = server_state.lock().await;
        state.phase = ServerPhase::Run;

        state.client_order = state.clients.keys().cloned().collect();
        state.current_player = 0;
        let current_cookie = state.client_order[0];

        match state.clients.get_mut(&current_cookie) {
            Some(game_sate) => {
                game_sate.phase = Chronology::GamePhase::Running;
                game_sate.should_wait = false;
                Ok(())
            }
            None => Err(ServerError::InvalidCookie),
        }
    }

    /// Starts a new game by generating events
    /// for the clients game state.
    ///
    /// **Note** that this function takes some time
    /// since it loads the datqbase. Therefore
    /// [`GameServer::restart_game`] should be
    /// called when restarting the game.
    async fn start_game(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        let mut state = server_state.lock().await;

        let game_state = server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;
        game_state.current_event = Some(game_state.game.generate_event()?);
        if game_state.phase == GamePhase::Running {
            send_message(stream, ToClient::StartIteration).await?;
        }

        Ok(())
    }

    /// Handles any comunication with the player
    /// during the run phase.
    async fn run_phase(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
    ) -> Result<(), ServerError> {
        loop {
            let message: ToServer = receive_message(stream).await?;

            match message {
                ToServer::Disconnect(cookie) => {
                    Self::disconnect(server_state.clone(), cookie).await?;
                }

                ToServer::Reconnect(cookie) => {
                    if let Err(e) = Self::reconnect(stream, server_state.clone(), cookie).await
                        && e != ServerError::InvalidCookie
                    {
                        return Err(e);
                    }
                }

                ToServer::RestartGame(cookie) => {
                    Self::restart_game(stream, server_state.clone(), cookie).await?
                }

                ToServer::RequestState(cookie) => {
                    Self::request_state(stream, server_state.clone(), cookie).await?
                }

                ToServer::Guess(cookie, guess_value) => {
                    Self::guess(stream, server_state.clone(), cookie, guess_value).await?
                }

                ToServer::IncorrectGuess(cookie) => {
                    Self::incorrect_guess(stream, server_state.clone(), cookie).await?
                }

                ToServer::Freeze(cookie, value) => {
                    Self::freeze(stream, server_state.clone(), cookie, value).await?
                }

                ToServer::RequestVictory(cookie) => {
                    Self::request_victory(stream, server_state.clone(), cookie).await?
                }

                ToServer::RestartIteration(cookie) => {
                    if Self::restart_iteration(stream, server_state.clone(), cookie).await? {
                        break;
                    }
                }

                invalid => {
                    Self::invalid_message(stream, invalid).await?;
                }
            }
        }
        Ok(())
    }

    /// Removes client state upon disconnection
    /// if the server phase is [`ServerPhase::Join`],
    /// otherwise the client will be marked as
    /// disconnected, but the stream will be keept.
    async fn disconnect(
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        let phase;
        println!("Disconnecting: {}", cookie);
        {
            let mut state = server_state.lock().await;
            phase = state.phase.clone();

            match phase {
                ServerPhase::Join => {
                    state.clients.remove(&cookie);
                    state.streams.remove(&cookie);
                }
                ServerPhase::Run => {
                    if let Some(game_state) = state.clients.get_mut(&cookie) {
                        game_state.client_disconnected = true;
                    }
                }
            }
        }

        match phase {
            ServerPhase::Join => server_utilities::send_player_list(server_state).await?,
            ServerPhase::Run => {
                if server_utilities::check_current_player(server_state.clone(), cookie).await {
                    server_utilities::set_next_player(server_state.clone(), cookie).await?
                }
            }
        }

        Ok(())
    }

    /// Reconnects client upon request, by sending
    /// confirmation along with the clients
    /// associated game state.
    async fn reconnect(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        let phase;
        {
            let mut state = server_state.lock().await;
            phase = state.phase.clone();

            let game_state =
                server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;

            if game_state.current_event.is_none() {
                game_state.game.reset_last_correct_index();
                game_state.client_disconnected = false;

                game_state.current_event = Some(game_state.game.generate_event()?);
            }

            println!("Server: Reconnection: {:?}", stream.peer_addr().unwrap());
            send_message(stream, ToClient::ReconnectAccepted(game_state.clone())).await?;

            state.streams.insert(cookie, stream.clone());
        }

        match phase {
            ServerPhase::Join => {
                return Err(ServerError::InvalidReconnect);
            }
            ServerPhase::Run => {
                server_utilities::handle_player_rejoin(stream, server_state.clone(), cookie).await?
            }
        }

        Ok(())
    }

    /// Restarts the clients game state. And
    /// generates new current event.
    async fn restart_game(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        let mut state = server_state.lock().await;

        let game_state = server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;
        game_state.game.restart()?;
        game_state.current_event = Some(game_state.game.generate_event()?);
        game_state.incorrect_event = None;

        Ok(())
    }

    /// Sends a copy of the current game state to
    /// the associated client.
    async fn request_state(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        let state = server_state.lock().await;
        let game_state = server_utilities::get_game_state(&state, cookie, stream).await?;
        let mut state_to_send = game_state.clone();

        if let Some(e) = &mut state_to_send.current_event {
            e.year = YearType::MAX; // Make sure correct result isn't sent.
        }

        send_message(stream, ToClient::State(state_to_send)).await?;
        Ok(())
    }

    /// Handles a provided guess from the client
    /// and updates the associated game state
    /// accordingly.
    async fn guess(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
        guess: Chronology::GuessType,
    ) -> Result<(), ServerError> {
        if !server_utilities::check_client_phase(server_state.clone(), cookie, GamePhase::Running)
            .await?
        {
            return Err(ServerError::InvalidClientPhase);
        }

        let mut state = server_state.lock().await;
        let game_state = server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;

        if let Some(event) = &game_state.current_event {
            game_state.game.reset_last_correct_index();
            if !game_state.game.add_guess(event.clone(), guess) {
                game_state.incorrect_event = Some(event.clone());
            }
        }
        game_state.current_event = None;

        Ok(())
    }

    /// Handles inccorect guess for the client.
    async fn incorrect_guess(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        if !server_utilities::check_client_phase(server_state.clone(), cookie, GamePhase::Running)
            .await?
        {
            return Err(ServerError::InvalidClientPhase);
        }

        let mut state = server_state.lock().await;
        let game_state = server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;

        game_state.game.incorrect_guess();
        game_state.should_wait = true;

        Ok(())
    }

    /// Freezes the clients board.
    async fn freeze(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
        value: bool,
    ) -> Result<(), ServerError> {
        if !server_utilities::check_client_phase(server_state.clone(), cookie, GamePhase::Running)
            .await?
        {
            return Err(ServerError::InvalidClientPhase);
        }

        if value {
            let mut state = server_state.lock().await;
            let game_state =
                server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;
            game_state.game.freeze_board();
            game_state.should_wait = true;
        }

        Ok(())
    }

    /// Sends a [`ToClient::VictoryResult`] message
    /// to the client, specifying if victory was
    /// achieved or not.
    async fn request_victory(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<(), ServerError> {
        if !server_utilities::check_client_phase(server_state.clone(), cookie, GamePhase::Running)
            .await?
        {
            return Err(ServerError::InvalidClientPhase);
        }

        let mut state = server_state.lock().await;
        let game_state = server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;
        send_message(
            stream,
            ToClient::VictoryResult(game_state.game.check_done()),
        )
        .await?;

        Ok(())
    }

    /// Restarts the current iteration of the game
    /// by updating the clients game state.
    async fn restart_iteration(
        stream: &mut TcpStream,
        server_state: Arc<Mutex<ServerState>>,
        cookie: Cookie,
    ) -> Result<bool, ServerError> {
        if !server_utilities::check_current_player(server_state.clone(), cookie).await {
            return Err(ServerError::NotCurrentPlayer);
        }

        if !server_utilities::check_client_phase(server_state.clone(), cookie, GamePhase::Running)
            .await?
        {
            return Err(ServerError::InvalidClientPhase);
        }

        let winner_name;
        let should_wait;

        {
            let mut state = server_state.lock().await;

            let game_state =
                server_utilities::get_game_state_mut(&mut state, cookie, stream).await?;

            should_wait = game_state.should_wait;

            game_state.game.reset_last_correct_index();
            game_state.current_event = Some(game_state.game.generate_event()?);
            game_state.incorrect_event = None;

            if game_state.game.check_done() {
                winner_name = Some(game_state.username.clone());
            } else {
                winner_name = None;
            }
        }

        if let Some(name) = winner_name {
            // Register victory
            server_utilities::send_message_to_all(server_state.clone(), ToClient::GameOver(name))
                .await?;

            //let mut state = server_state.lock().await;
            //state.phase = ServerPhase::Restarting;
            Self::reset_server_state(server_state).await;
            return Ok(true);
        } else if should_wait {
            server_utilities::set_next_player(server_state.clone(), cookie).await?;
        } else {
            // Continue with same client
            send_message(stream, ToClient::StartIteration).await?;
        }

        Ok(false)
    }

    /// Sends an invalid message to the client
    /// and generates an error.
    async fn invalid_message(stream: &mut TcpStream, message: ToServer) -> Result<(), ServerError> {
        println!(
            "Message discurraged, invalid message for this server phase. Message: {:?}",
            message
        );
        send_message(stream, ToClient::InvalidMessage).await?;
        Err(ServerError::InvalidMessage)
    }

    async fn reset_server_state(server_state: Arc<Mutex<ServerState>>) {
        println!("Resetting server state");
        let mut state = server_state.lock().await;
        *state = ServerState::new();
    }
}

#[cfg(test)]
mod server_tests {
    use super::*;
    use smol::Timer;
    use std::collections::HashMap;

    #[test]
    fn start_server_test() {
        let ip = String::from("127.0.0.4");
        let port = String::from("8080");

        let mut server = GameServer::new(&ip.clone(), &port.clone());

        assert_eq!(server.ip, ip);
        assert_eq!(server.port, port);

        server.start(false);

        // Let server start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Start client
        smol::block_on(async {
            let client = smol::spawn(async move {
                let _ = TcpStream::connect(format!("{0}:{1}", ip, port))
                    .await
                    .expect("Failed to connect to server");
            });

            client.await;
        });
    }

    #[test]
    fn connect_test() {
        let ip = String::from("127.0.0.4");
        let port = String::from("8081");

        let mut server = GameServer::new(&ip.clone(), &port.clone());
        server.start(false);

        // Let server start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Start client
        smol::block_on(async {
            let client = smol::spawn(async move {
                let mut stream = TcpStream::connect(format!("{0}:{1}", ip, port))
                    .await
                    .expect("Failed to connect to server");

                // Send connect
                send_message(&mut stream, ToServer::Connect(String::new()))
                    .await
                    .expect("Failed sending message");

                // Receive connect accepted
                let ToClient::ConnectionAccepted(_) = receive_message::<ToClient>(&mut stream)
                    .await
                    .expect("Failed to read message")
                else {
                    panic!("Expected ConnectionAccepted");
                };

                let state = server.server_state.lock().await;
                assert_eq!(state.clients.len(), 1);
            });

            client.await;
        });
    }

    #[test]
    fn start_game_test() {
        let ip = String::from("127.0.0.4");
        let port = String::from("8084");

        let mut server = GameServer::new(&ip.clone(), &port.clone());
        server.start(false);

        // Let server start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Start first client
        smol::block_on(async {
            let client = smol::spawn(async move {
                let mut stream = TcpStream::connect(format!("{0}:{1}", ip, port))
                    .await
                    .expect("Failed to connect to server");

                // Send connect
                send_message(&mut stream, ToServer::Connect(String::new()))
                    .await
                    .expect("Failed sending message");

                // Receive connect accepted
                let ToClient::ConnectionAccepted(cookie) = receive_message::<ToClient>(&mut stream)
                    .await
                    .expect("Failed to read message")
                else {
                    panic!("Expected ConnectionAccepted");
                };

                send_message(&mut stream, ToServer::StartGame(cookie))
                    .await
                    .expect("Failed sending message");

                // Give server time to respond
                Timer::after(std::time::Duration::from_millis(100)).await;

                let state = server.server_state.lock().await;
                assert_ne!(
                    state
                        .clients
                        .get(&cookie)
                        .expect("Failed to get client")
                        .current_event,
                    None
                );
            });

            client.await;
        });
    }

    #[test]
    fn restart_game_test() {
        let ip = String::from("127.0.0.4");
        let port = String::from("8085");

        let mut server = GameServer::new(&ip.clone(), &port.clone());
        server.start(false);

        // Let server start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Start client
        smol::block_on(async {
            let client = smol::spawn(async move {
                let mut stream = TcpStream::connect(format!("{0}:{1}", ip, port))
                    .await
                    .expect("Failed to connect to server");

                // Send connect
                send_message(&mut stream, ToServer::Connect(String::new()))
                    .await
                    .expect("Failed sending message");

                // Receive connect accepted
                let ToClient::ConnectionAccepted(cookie) = receive_message::<ToClient>(&mut stream)
                    .await
                    .expect("Failed to read message")
                else {
                    panic!("Expected ConnectionAccepted");
                };

                send_message(&mut stream, ToServer::StartGame(cookie))
                    .await
                    .expect("Failed sending message");

                send_message(&mut stream, ToServer::RestartGame(cookie))
                    .await
                    .expect("Failed sending message");

                // Give server time to respond
                Timer::after(std::time::Duration::from_millis(100)).await;

                let state = server.server_state.lock().await;
                assert_eq!(
                    state
                        .clients
                        .get(&cookie)
                        .expect("Failed to get client")
                        .game
                        .get_round(),
                    1
                );
            });

            client.await;
        });
    }

    #[test]
    fn disconnect_test() {
        let stream = smol::block_on(async {
            let port = "2000";
            accept_clients(1, port).await;
            create_client(port).await
        });

        smol::block_on(async move {
            let s = stream.clone();
            let test_cookie = 141231;

            let join_state = create_server_state(
                HashMap::from([(test_cookie, craete_gamestate())]),
                HashMap::from([(test_cookie, s)]),
                ServerPhase::Join,
            );

            let res = GameServer::disconnect(join_state.clone(), test_cookie).await;
            assert!(res.is_ok());
            let state = join_state.lock().await;
            assert_eq!(state.clients.len(), 0);
        });

        //assert!(matches!(receive_message(&mut stream).await, Ok(ToClient::PlayerList(_))));
    }

    async fn accept_clients(n_clients: u8, port: &str) {
        let ip = "127.0.0.4";
        let listener = TcpListener::bind(format!("{0}:{1}", ip, port))
            .await
            .expect("Failed binding server");

        smol::spawn(async move {
            for _ in 0..n_clients {
                listener.accept().await.expect("Failed accepting client");
            }
        })
        .detach();
    }

    async fn create_client(port: &str) -> TcpStream {
        let ip = "127.0.0.4";
        TcpStream::connect(format!("{0}:{1}", ip, port))
            .await
            .expect("Failed to connect to server")
    }

    fn create_server_state(
        clients: HashMap<Cookie, GameState>,
        streams: HashMap<Cookie, TcpStream>,
        phase: ServerPhase,
    ) -> Arc<Mutex<ServerState>> {
        Arc::new(Mutex::new(ServerState {
            clients,
            streams,
            phase,
            client_order: Vec::new(),
            current_player: 0,
            session_owner: None,
        }))
    }

    fn craete_gamestate() -> GameState {
        GameState {
            game: ChronologyGame::new(),
            current_event: None,
            incorrect_event: None,
            username: String::from("test"),
            should_wait: true,
            phase: Chronology::GamePhase::Waiting,
            client_disconnected: false,
        }
    }
}
