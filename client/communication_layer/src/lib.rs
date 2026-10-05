//! # Communication Layer
//!
//! This module contains functionallity for
//! establishing communication between two
//! systems. Where one system runs
//! asynchronously while the other runs
//! synchronously. More specifically
//! this implementation is specific to
//! the communication between a client/
//! backend and a UI/frontend.

use async_channel::{Receiver, Sender, unbounded};
pub use async_channel::{RecvError, SendError, TryRecvError, TrySendError};

use bevy::prelude::Resource;
pub use game_common::Chronology;

/// Messages that can be send from the client/backend
/// to the UI/frontend.
#[derive(Debug, Clone)]
pub enum ClientToUi {
    Start(Chronology::YearType),
    Wait,
    SetEvent(Chronology::Event),
    Guess,
    AskFreeze,
    Result(bool, Chronology::YearType),
    SetBoard(Chronology::ChronologyGame),
    Victory(String),

    Shutdown,
}

/// Messages that can be send from the UI/frontend
/// to the client/backend.
#[derive(Debug, Clone)]
pub enum UiToClient {
    Guess(Chronology::GuessType),
    Freeze(bool),
    Continue,
}

/// Resource wrapper for [`UiComLayer`] .
#[derive(Resource)]
pub struct CommunicationLayer {
    pub com_layer: UiComLayer,
}

/// Communication layer for communication from
/// client/backend to UI/frontend.
#[derive(Clone)]
pub struct ClientComLayer {
    sender: Sender<ClientToUi>,
    receiver: Receiver<UiToClient>,
}

impl ClientComLayer {
    /// Returns a new instance of [`ClientComLayer`].
    pub fn new(sender: Sender<ClientToUi>, receiver: Receiver<UiToClient>) -> Self {
        Self { sender, receiver }
    }

    /// Sends a message to the UI/frontend.
    pub async fn send(&self, message: ClientToUi) -> Result<(), SendError<ClientToUi>> {
        #[cfg(debug_assertions)]
        println!("Client -> UI: {:?}", message);

        self.sender.send(message).await
    }

    /// Receives a message from the UI/frontend.
    pub async fn receive(&self) -> Result<UiToClient, RecvError> {
        self.receiver.recv().await
    }
}

/// Communication layer for communication from
/// UI/frontend to client/backend.
#[derive(Clone)]
pub struct UiComLayer {
    sender: Sender<UiToClient>,
    receiver: Receiver<ClientToUi>,
}

impl UiComLayer {
    /// Returns a new instance of [`UiComLayer`].
    pub fn new(sender: Sender<UiToClient>, receiver: Receiver<ClientToUi>) -> Self {
        Self { sender, receiver }
    }

    /// Sends a message to the client/backend.
    pub fn send(&self, message: UiToClient) -> Result<(), TrySendError<UiToClient>> {
        #[cfg(debug_assertions)]
        println!("UI -> Client: {:?}", message);

        self.sender.try_send(message)
    }

    /// Receives a message from the client/backend.
    pub fn receive(&self) -> Result<ClientToUi, TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Creates two communication layers, one for
/// the UI/frontend and one for the client/backend.
pub fn create_com_layers() -> (UiComLayer, ClientComLayer) {
    let (tx_client_ui, rx_client_ui) = unbounded::<ClientToUi>();
    let (tx_ui_client, rx_ui_client) = unbounded::<UiToClient>();

    (
        UiComLayer::new(tx_ui_client, rx_client_ui),
        ClientComLayer::new(tx_client_ui, rx_ui_client),
    )
}
