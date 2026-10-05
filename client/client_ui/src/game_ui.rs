//! # GameUI
//!
//! Provides functionality for running a UI
//! application of the cronology gane. The UI runs
//! using [`bevy`].

use bevy::picking::PickingSystems;
use bevy::prelude::*;

use communication_layer::*;

use crate::game_definitions::*;
use crate::game_systems::systems;
use crate::game_utilities::*;

/// Datastructure for the UI wich contains a
/// bevy application.
pub struct GameUI {
    app: App,
}

impl GameUI {
    /// Create a new instance of [`GameUI`]. With
    /// a configues bevy application.
    pub fn new(com_layer: UiComLayer) -> Self {
        let mut app = App::new();

        app.add_plugins(DefaultPlugins);

        // Configure state and stes
        app.init_state::<GameState>();
        app.configure_sets(
            Update,
            GuessingSet
                .run_if(in_state(GameState::Guessing))
                .run_if(not(in_state(GameState::GameOver))),
        );
        app.configure_sets(
            PostUpdate,
            GuessingSet
                .run_if(in_state(GameState::Guessing))
                .run_if(not(in_state(GameState::GameOver))),
        );
        app.configure_sets(Update, GameSet.run_if(not(in_state(GameState::GameOver))));

        // Startup
        app.add_systems(
            Startup,
            (
                systems::setup,
                systems::disable_interactive_ui.after(systems::setup),
            ),
        );

        // Systems that will run in all states
        app.add_systems(First, drive_diegetic_pointer.in_set(PickingSystems::Input));
        app.add_systems(PreUpdate, systems::receive_client_messages);

        app.add_systems(
            Update,
            (
                systems::main_update,
                systems::update_text_quad,
                systems::mute,
            ),
        );

        // Debug systems
        if cfg!(debug_assertions) {
            app.add_systems(Update, move_system_debug);
        }

        // Game related systems
        app.add_systems(
            Update,
            (
                systems::add_placed_card,
                systems::freeze_board,
                systems::clear_loose_cards,
                systems::set_board,
                systems::update_card_labels,
            )
                .in_set(GameSet),
        );

        // Systems for Guessing state
        app.add_systems(
            Update,
            (systems::add_active_card, systems::update_active_card).in_set(GuessingSet),
        );

        app.add_systems(PostUpdate, systems::update_board);

        // Systems for FreezeOption state
        app.add_systems(
            OnEnter(GameState::FreezeOption),
            systems::enable_interactive_ui,
        );
        app.add_systems(
            OnExit(GameState::FreezeOption),
            systems::disable_interactive_ui,
        );

        // Systems for ContinueOption state
        app.add_systems(
            OnEnter(GameState::ContinueOption),
            systems::enable_interactive_ui_continue,
        );
        app.add_systems(
            OnExit(GameState::ContinueOption),
            systems::disable_interactive_ui,
        );

        // Systems for GameOVer
        app.add_systems(OnEnter(GameState::GameOver), systems::remove_all_cards);

        // Observers
        app.add_observer(systems::freeze_button_observer);

        // Resources
        app.insert_resource(CommunicationLayer { com_layer });

        // Messages
        app.add_message::<NewActiveCard>();
        app.add_message::<NewPlacedCard>();
        app.add_message::<IncorrectGuess>();
        app.add_message::<UpdateTextQuad>();
        app.add_message::<FreezeBoard>();
        app.add_message::<ClearLooseCards>();
        app.add_message::<SetBoard>();

        Self { app }
    }

    /// Run the game.
    pub fn run(&mut self) {
        self.app.run();
    }
}
