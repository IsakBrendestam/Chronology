//! # Message Definitions
//!
//! Definitions related to message communication
//! between server and client.

use serde::{Deserialize, Serialize};
use smol::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use std::fmt::Debug;

use crate::chronology_game::game as Chronology;
use Chronology::ChronologyGame;

pub type Cookie = u64;

/// Stores the sate of the game together with
/// the current event.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct GameState {
    pub game: ChronologyGame,
    pub current_event: Option<Chronology::Event>,
    pub incorrect_event: Option<Chronology::Event>,
    pub username: String,
    pub should_wait: bool,
    pub phase: Chronology::GamePhase,
    pub client_disconnected: bool,
}

/// Messages possible to send to the server.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ToServer {
    Connect(String),
    Disconnect(Cookie),
    Reconnect(Cookie),
    StartSession(Cookie),

    ReadyToStart(Cookie),
    StartGame(Cookie),
    RestartGame(Cookie),
    RequestState(Cookie),
    Guess(Cookie, Chronology::GuessType),
    IncorrectGuess(Cookie),
    Freeze(Cookie, bool),
    RequestVictory(Cookie),
    RestartIteration(Cookie),
}

/// Messages possible to send to the client.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ToClient {
    ConnectionAccepted(Cookie),
    ConnectionRejected,
    ReconnectAccepted(GameState),
    RejoinSession,
    DisconnectConfirmed,

    SessionOwner,
    PlayerList(Vec<String>),

    StartSession,
    StartIteration,

    State(GameState),
    VictoryResult(bool),
    GameOver(String), // Winner name
    InvalidMessage,
    InvalidCookie,
}

pub trait MessageType {}
impl MessageType for ToClient {}
impl MessageType for ToServer {}

/// Reads a message of type `T` from `stream`, where
/// the first four bytes represents the length of
/// the message in bytes.
pub async fn receive_message<T>(stream: &mut TcpStream) -> Result<T, smol::io::Error>
where
    T: serde::de::DeserializeOwned + MessageType + Debug,
{
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut buf = vec![0; len];
    stream.read_exact(&mut buf).await?;
    let res = serde_json::from_slice(&buf[0..len])?;

    #[cfg(debug_assertions)]
    println!("Receiving: {:?}", res);

    Ok(res)
}

/// Sends a message of type `T` to `stream`. Where
/// the first four bytes represents the length of
/// the message in bytes.
pub async fn send_message<T>(stream: &mut TcpStream, message: T) -> Result<(), smol::io::Error>
where
    T: serde::Serialize + MessageType,
{
    let msg = serde_json::to_string(&message)?;
    let len = msg.len() as u32;

    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(msg.as_bytes()).await?;

    #[cfg(debug_assertions)]
    println!("Sending: {:?}", msg);

    Ok(())
}

#[cfg(test)]
mod message_tests {
    use super::*;
    use smol::net::TcpListener;

    #[test]
    fn send_and_recieve_message_test() {
        let time_limit = 2;

        let ip = "127.0.0.2";
        let port = "8080";

        let test_cookie = 10;

        smol::block_on(async move {
            timeout(
                async {
                    let server = smol::spawn(async move {
                        let tcp_listener = TcpListener::bind(format!("{0}:{1}", ip, port))
                            .await
                            .expect("Failed to bind server.");

                        let (mut stream, _) = tcp_listener
                            .accept()
                            .await
                            .expect("Fialed to accept client.");

                        send_message(&mut stream, ToClient::ConnectionAccepted(test_cookie))
                            .await
                            .expect("Failed to send connection accepted message");
                    });

                    // Let server start
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    let client = smol::spawn(async move {
                        let mut stream = TcpStream::connect(format!("{0}:{1}", ip, port))
                            .await
                            .expect("Failed to connect to server");

                        let message: ToClient = receive_message(&mut stream)
                            .await
                            .expect("Failed to read message.");

                        assert_eq!(message, ToClient::ConnectionAccepted(test_cookie));
                    });

                    client.await;
                    server.await;
                },
                time_limit,
            )
            .await;
        });
    }

    #[test]
    fn to_client_messages_test() {
        let time_limit = 2;

        let ip = "127.0.0.2";
        let port = "8081";

        let server_cases = vec![
            ToClient::ConnectionAccepted(10),
            ToClient::ConnectionAccepted(5),
            ToClient::ConnectionRejected,
            ToClient::ReconnectAccepted(GameState {
                game: ChronologyGame::new(),
                current_event: None,
                incorrect_event: None,
                username: String::from("Test"),
                should_wait: false,
                phase: Chronology::GamePhase::Waiting,
                client_disconnected: false,
            }),
            ToClient::State(GameState {
                game: ChronologyGame::new(),
                current_event: None,
                incorrect_event: None,
                username: String::from("Test 1"),
                should_wait: true,
                phase: Chronology::GamePhase::Running,
                client_disconnected: false,
            }),
        ];
        let client_cases = server_cases.clone();

        smol::block_on(async move {
            timeout(
                async {
                    // Run Server
                    let server = smol::spawn(async move {
                        let tcp_listener = TcpListener::bind(format!("{0}:{1}", ip, port))
                            .await
                            .expect("Failed to bind server.");

                        let (mut stream, _) = tcp_listener
                            .accept()
                            .await
                            .expect("Fialed to accept client.");

                        for expected in server_cases {
                            send_message(&mut stream, expected)
                                .await
                                .expect("Failed to send connection accepted message");
                        }
                    });

                    // Let server start
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    // Run Client
                    let client = smol::spawn(async move {
                        let mut stream = TcpStream::connect(format!("{0}:{1}", ip, port))
                            .await
                            .expect("Failed to connect to server");

                        for expected in client_cases {
                            let message: ToClient = receive_message(&mut stream)
                                .await
                                .expect("Failed to read message.");
                            assert_eq!(expected, message);
                        }
                    });

                    client.await;
                    server.await;
                },
                time_limit,
            )
            .await;
        });
    }

    #[test]
    fn to_server_messages_test() {
        let time_limit = 2;

        let ip = "127.0.0.2";
        let port = "8082";

        let test_cookie = 10;

        let server_cases = vec![
            ToServer::Connect(String::from("Test")),
            ToServer::Disconnect(test_cookie),
            ToServer::Reconnect(test_cookie),
            ToServer::StartGame(test_cookie),
            ToServer::RestartGame(test_cookie),
            ToServer::RequestState(test_cookie),
            ToServer::Guess(test_cookie, 7),
            ToServer::Guess(test_cookie, 113),
            ToServer::IncorrectGuess(test_cookie),
            ToServer::Freeze(test_cookie, false),
            ToServer::Freeze(test_cookie, true),
            ToServer::RestartIteration(test_cookie),
        ];
        let client_cases = server_cases.clone();

        smol::block_on(async move {
            timeout(
                async {
                    let server_cases_clone = server_cases.clone();
                    let client_cases_clone = client_cases.clone();

                    // Run Server
                    let server = smol::spawn(async move {
                        let tcp_listener = TcpListener::bind(format!("{0}:{1}", ip, port))
                            .await
                            .expect("Failed to bind server.");

                        let (mut stream, _) = tcp_listener
                            .accept()
                            .await
                            .expect("Fialed to accept client.");

                        for expected in server_cases_clone {
                            let message: ToServer = receive_message(&mut stream)
                                .await
                                .expect("Failed to read message.");
                            assert_eq!(expected, message);
                        }
                    });

                    // Let server start
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    // Run Client
                    let client = smol::spawn(async move {
                        let mut stream = TcpStream::connect(format!("{0}:{1}", ip, port))
                            .await
                            .expect("Failed to connect to server");

                        for expected in client_cases_clone {
                            send_message(&mut stream, expected)
                                .await
                                .expect("Failed to send connection accepted message");
                        }
                    });

                    client.await;
                    server.await;
                },
                time_limit,
            )
            .await;
        });
    }

    async fn timeout<F: std::future::Future>(func: F, seconds: u64) -> F::Output {
        smol::future::race(func, async {
            smol::Timer::after(std::time::Duration::from_secs(seconds)).await;
            panic!("Operation timed out");
        })
        .await
    }
}
