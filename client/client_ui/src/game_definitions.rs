//! A collection of definitions used in the game.
//! This file contains the following categories:
//!
//! * Components
//! * States
//! * Sets
//! * Internal Messages
//! * Resources

use std::cmp::Ordering;

use bevy::prelude::*;
use game_common::Chronology;

// ---------- Components ----------

#[derive(Component)]
pub struct MoveComp;

#[derive(Component)]
pub struct ActiveCard;

#[derive(Component)]
pub struct LooseCard;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct Card {
    pub year: Chronology::YearType,
}

#[derive(Component)]
pub struct TextQuad {
    pub text_entity: Entity,
}

#[derive(Component)]
pub struct InteractiveUiQuad;

#[derive(Component)]
pub struct InteractiveUiCamera;

#[derive(Component)]
pub struct CardLabel {
    pub target: Entity,
}

#[derive(Component, PartialEq)]
pub enum FreezeOptionButton {
    Continue,
    Freeze,
}

#[derive(Component)]
pub struct FreezeOptionBase;

#[derive(Component)]
pub struct AudioControll;

// ---------- States ----------

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Waiting,
    Guessing,
    FreezeOption,
    ContinueOption,
    GameOver,
}

// ---------- Sets ----------

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct GuessingSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct GameSet;

// ---------- Internal Messages ----------

#[derive(Message)]
pub struct NewActiveCard {
    pub year: Chronology::YearType,
}

#[derive(Message)]
pub struct NewPlacedCard {
    pub year: Chronology::YearType,
    pub frozen: bool,
}

#[derive(Message)]
pub struct IncorrectGuess;

#[derive(Message)]
pub struct FreezeBoard;

#[derive(Message)]
pub struct ClearLooseCards;

#[derive(Message)]
pub struct SetBoard {
    pub events: Vec<Chronology::GameEventWrapper>,
}

#[derive(Message)]
pub struct UpdateTextQuad {
    pub parent: Entity,
    pub text: String,
}

// ---------- Resources ----------

#[derive(Resource)]
pub struct GameSounds {
    place: Handle<AudioSource>,
    clear: Handle<AudioSource>,
    freeze: Handle<AudioSource>,
}

impl GameSounds {
    pub fn load_sounds(commands: &mut Commands, asset_server: &Res<AssetServer>) {
        commands.insert_resource(GameSounds {
            place: asset_server.load("sounds/place_card.ogg"),
            clear: asset_server.load("sounds/incorrect.ogg"),
            freeze: asset_server.load("sounds/freeze.ogg"),
        });
    }

    pub fn play_place(commands: &mut Commands, sounds: &Res<GameSounds>) {
        play_sound(commands, sounds.place.clone());
    }

    pub fn play_clear(commands: &mut Commands, sounds: &Res<GameSounds>) {
        play_sound(commands, sounds.clear.clone());
    }

    pub fn play_freeze(commands: &mut Commands, sounds: &Res<GameSounds>) {
        play_sound(commands, sounds.freeze.clone());
    }
}

fn play_sound(commands: &mut Commands, sound: Handle<AudioSource>) {
    commands.spawn((AudioPlayer::new(sound), PlaybackSettings::DESPAWN));
}

#[derive(Resource)]
pub struct BoardState {
    pub min: Vec3,
    pub max: Vec3,
    pub center: Vec3,
    pub dirty: bool,
    pub cards: Vec<Entity>,
    pub display: Entity,
}

impl BoardState {
    pub fn new(min: Vec3, max: Vec3, center: Vec3, display: Entity) -> Self {
        Self {
            min,
            max,
            center,
            dirty: true,
            cards: Vec::new(),
            display,
        }
    }

    pub fn add_card(&mut self, card: Entity) {
        if self.cards.len() < 10 {
            self.dirty = true;
            self.cards.push(card);
        }
    }

    pub fn remove_card(&mut self, card: Entity) {
        if let Some(index) = self.cards.iter().position(|c| *c == card) {
            self.dirty = true;
            self.cards.remove(index);
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn sort(
        &mut self,
        query: &Query<(&mut Transform, &Card), (With<Card>, Without<ActiveCard>)>,
    ) {
        if self.dirty {
            self.cards.sort_by(|a, b| {
                if let Ok((_, a_card)) = query.get(*a)
                    && let Ok((_, b_card)) = query.get(*b)
                {
                    if a_card.year > b_card.year {
                        return Ordering::Greater;
                    }
                    if a_card.year < b_card.year {
                        return Ordering::Less;
                    }
                    return Ordering::Equal;
                }
                Ordering::Greater
            });
            self.dirty = false;
        }
    }

    pub fn generate_board_positions(&self, n_positions: usize) -> Vec<Vec3> {
        let mut positions = Vec::with_capacity(self.cards.len());

        let n = n_positions as f32;
        let d = Vec3::distance(self.min, self.max);

        for index in 0..n_positions {
            let i = index as f32;
            let lambda_i = (i + 1.0) * 1.0 / (n + 1.0);

            positions.push(self.min - Vec3::new(0.0, 0.0, d * lambda_i));
        }

        positions
    }

    pub fn min_dist_index(&self, position: Vec3, board_positions: &[Vec3]) -> Option<usize> {
        let mut min_dist = f32::MAX;
        let mut index = self.cards.len() + 1;
        let mut found = false;

        for (i, pos) in board_positions.iter().enumerate() {
            let distance = Vec3::distance(*pos, position);
            if distance < min_dist && distance < 1.5 {
                min_dist = distance;
                index = i;
                found = true;
            }
        }

        if found {
            return Some(index);
        }
        None
    }
}
