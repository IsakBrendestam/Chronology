//! Systems used for the game. These systems are
//! passed to Bevy in [`game_ui.rs`].

pub mod systems {
    use core::f32;

    use bevy::audio::PlaybackMode;
    use bevy::ecs::entity_disabling::Disabled;
    use bevy::{app::AppExit, prelude::*, window::PrimaryWindow};

    use communication_layer::*;

    use crate::custom_3d_ui::ui_3d;
    use crate::game_definitions::*;
    use crate::game_utilities::{format_display_text, get_mouse_board_position};

    /// Setup scene by spawning entities for
    /// meshes, lights, camera, resources and
    /// audio.
    pub fn setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let static_scene: Handle<Scene> = asset_server.load("meshes/static_enviroment.glb#Scene0");
        commands.spawn((SceneRoot(static_scene), Transform::default()));

        let board_min = Vec3::new(0.0, 5.001, 4.0);
        let board_max = Vec3::new(0.0, 5.001, -4.0);
        let board_center = Vec3::new(0.0, 5.0, 0.0);

        let display = commands
            .spawn(Transform::from_xyz(-6.5979753, 6.25, -5.575057))
            .id();

        commands.insert_resource(BoardState::new(board_min, board_max, board_center, display));

        let base_rot = Quat::from_euler(
            EulerRot::XYZ,
            f32::consts::PI,
            f32::consts::PI - f32::consts::PI / 6.0,
            0.0,
        );

        let normal = base_rot * Vec3::Z;
        let rotation = Quat::from_axis_angle(normal, f32::consts::PI) * base_rot;

        let data = ui_3d::TextQuadData {
            parent: display,
            font_size: 200.0,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                rotation,
                scale: Vec3::new(7.5, 7.5, 1.0),
            },
            text: &format_display_text("Category", "Event Text"),
            font: None,
        };

        ui_3d::add_text_quad(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            data,
        );

        let camera_transform = Transform {
            translation: Vec3::new(9.85115, 11.754351, 1.326515),
            rotation: Quat::from_vec4(Vec4::new(-0.16605318, 0.5917413, 0.12618822, 0.77868146)),
            scale: Vec3::new(1.0, 1.0, 1.0),
        };

        let freeze_base = commands
            .spawn((
                Transform::from_xyz(1.736203, 5.0, 0.0),
                FreezeOptionBase,
                Visibility::Hidden,
            ))
            .id();

        // Quad transform relative to parent
        let quad_transform = Transform {
            translation: Vec3::ZERO,         // relative to parent
            scale: Vec3::new(4.0, 5.0, 1.0), // scale in
            ..default()
        };

        ui_3d::add_freeze_option_quad(
            &mut commands,
            freeze_base,
            quad_transform.looking_to(Vec3::new(0.0, -1.0, 0.0), Vec3::Y),
            &mut meshes,
            &mut materials,
            &mut images,
        );

        // Camera in 3D space.
        commands.spawn((Camera3d::default(), camera_transform, MainCamera));

        // Add lights
        let light = PointLight {
            intensity: 4_000_000.0,
            color: Color::WHITE,
            range: 20.0,
            radius: 20.0,
            shadows_enabled: true,
            ..default()
        };
        commands.spawn((light, Transform::from_xyz(0.0, 7.5, 0.0)));

        let light = SpotLight {
            intensity: 4_000_000.0,
            color: Color::WHITE,
            range: 20.0,
            radius: 20.0,
            shadows_enabled: true,
            ..default()
        };
        commands.spawn((light, Transform::from_xyz(0.0, 7.5, 0.0)));

        // Add sound
        GameSounds::load_sounds(&mut commands, &asset_server);

        commands.spawn((
            AudioPlayer::new(asset_server.load("sounds/541322__dblover__squeaky-wooden-floor.ogg")),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                ..default()
            },
            AudioControll,
        ));
    }

    /// Receives messages though the
    /// [`CommunicationLayer`] and takes
    /// appropriate actions.
    #[allow(clippy::too_many_arguments)]
    pub fn receive_client_messages(
        com: Res<CommunicationLayer>,
        board: ResMut<BoardState>,
        mut game_state: ResMut<NextState<GameState>>,
        mut card_writer: MessageWriter<NewPlacedCard>,
        mut active_card_writer: MessageWriter<NewActiveCard>,
        mut clear_writer: MessageWriter<ClearLooseCards>,
        mut text_writer: MessageWriter<UpdateTextQuad>,
        mut set_board_writer: MessageWriter<SetBoard>,
        mut exit: MessageWriter<AppExit>,
    ) {
        while let Ok(msg) = com.com_layer.receive() {
            match msg {
                ClientToUi::Start(year) => {
                    card_writer.write(NewPlacedCard { year, frozen: true });
                }
                ClientToUi::SetEvent(event) => {
                    text_writer.write(UpdateTextQuad {
                        parent: board.display,
                        text: format_display_text(&event.category, &event.text),
                    });
                }
                ClientToUi::Wait => {
                    game_state.set(GameState::Waiting);
                    text_writer.write(UpdateTextQuad {
                        parent: board.display,
                        text: String::from("Waiting..."),
                    });
                }
                ClientToUi::Guess => {
                    // Draw new card
                    active_card_writer.write(NewActiveCard { year: 0 });
                    game_state.set(GameState::Guessing);
                }
                ClientToUi::AskFreeze => {
                    game_state.set(GameState::FreezeOption);
                }
                ClientToUi::Result(res, year) => {
                    if res {
                        card_writer.write(NewPlacedCard {
                            year,
                            frozen: false,
                        });
                        text_writer.write(UpdateTextQuad {
                            parent: board.display,
                            text: String::from("Correct!"),
                        });
                    } else {
                        text_writer.write(UpdateTextQuad {
                            parent: board.display,
                            text: format_display_text("Incorrect", &format!("Year was: {}", year)),
                        });
                        clear_writer.write(ClearLooseCards);

                        game_state.set(GameState::ContinueOption);
                    }
                }
                ClientToUi::SetBoard(game_state) => {
                    set_board_writer.write(SetBoard {
                        events: game_state.get_active_events(),
                    });
                }
                ClientToUi::Victory(name) => {
                    text_writer.write(UpdateTextQuad {
                        parent: board.display,
                        text: format!("Game Over!\n\nWinner:\n{}", name),
                    });
                    game_state.set(GameState::GameOver);
                }
                ClientToUi::Shutdown => {
                    exit.write(AppExit::Success);
                }
            }
        }
    }

    /// Main update system of the game, that's mainly
    /// used for debuging.
    pub fn main_update(keyboard: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
        if keyboard.just_pressed(KeyCode::Escape) {
            exit.write(AppExit::Success);
        }
    }

    /// Places a card on the baord, this is used
    /// when starting the game or when reconnecting.
    /// It's invoked by sending a [`NewPlacedCard`]
    /// message.
    pub fn add_placed_card(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut board: ResMut<BoardState>,
        mut reader: MessageReader<NewPlacedCard>,
        sounds: Res<GameSounds>,
    ) {
        for card in reader.read() {
            let card_mesh = if card.frozen {
                asset_server.load("meshes/card_frozen.glb#Scene0")
            } else {
                asset_server.load("meshes/card_active.glb#Scene0")
            };

            let new_card = commands
                .spawn((
                    Card { year: card.year },
                    SceneRoot(card_mesh),
                    Transform::from_xyz(0.0, 5.0, 0.0),
                ))
                .id();

            if !card.frozen {
                commands.entity(new_card).insert(LooseCard);
            }

            board.add_card(new_card);

            commands.spawn((
                Text::new(format!("{}", card.year)),
                TextFont {
                    font: asset_server.load("fonts/GreatVibes-Regular.ttf"),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                CardLabel { target: new_card },
            ));

            GameSounds::play_place(&mut commands, &sounds);
        }
    }

    /// Spawns a active card that will follow the
    /// mouse cursor.
    pub fn add_active_card(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut reader: MessageReader<NewActiveCard>,
    ) {
        for card in reader.read() {
            let card_mesh: Handle<Scene> = asset_server.load("meshes/card_active.glb#Scene0");

            commands.spawn((
                Card { year: card.year },
                ActiveCard,
                SceneRoot(card_mesh),
                Transform::from_xyz(0.0, 5.2, 0.0),
            ));
        }
    }

    /// Makes active card follow the cursos. The
    /// card will be limited to only follow the
    /// cursor within a ellips that represents
    /// the baord.
    ///
    /// Also note that the target position is
    /// slightly elevated in the y-axis.
    pub fn update_active_card(
        mut query: Query<(Entity, &mut Transform), With<ActiveCard>>,
        board: ResMut<BoardState>,
        window: Single<&Window, With<PrimaryWindow>>,
        camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
        time: Res<Time>,
    ) {
        for (_, mut transform) in &mut query {
            if let Some(target) = get_mouse_board_position(&window, camera) {
                let target = target + Vec3::new(0.0, 0.2, 0.0);

                // Limit the card to an ellipse
                let width = 3.4;
                let height = 2.2;
                let board_center = board.center;

                let to_target = target - board_center;

                let scaled = Vec3::new(to_target.x / height, 0.0, to_target.z / width);

                let scaled_len = scaled.length();

                let clamped_target = if scaled_len > 1.0 {
                    let normalized = scaled / scaled_len;
                    board_center + Vec3::new(normalized.x * height, 0.0, normalized.z * width)
                } else {
                    target
                };

                // Move the card smoothly
                let delta = clamped_target - transform.translation;
                transform.translation += delta * 25.0 * time.delta_secs();
            }
        }
    }

    /// Updates the card positions on the board,
    /// and moves them according the the interaction
    /// with the active card.
    ///
    /// A input check is also performed to register
    /// if the player made a guess, and what the
    /// index was in that case.
    #[allow(clippy::type_complexity)]
    pub fn update_board(
        mut commands: Commands,
        mouse: Res<ButtonInput<MouseButton>>,
        mut query: Query<(&mut Transform, &Card), (With<Card>, Without<ActiveCard>)>,
        query_active: Query<(Entity, &mut Transform), With<ActiveCard>>,
        mut board: ResMut<BoardState>,
        com: ResMut<CommunicationLayer>,
    ) {
        // Sort cards
        board.sort(&query);

        // Calculate closest board position
        let mut min_dist_index = board.cards.len() + 1;

        let active_card = query_active.single();

        let extra = active_card.is_ok() as usize;
        let mut board_positions = board.generate_board_positions(board.cards.len() + extra);

        let mut found = false;

        if let Ok((_, active_pos)) = active_card
            && let Some(index) = board.min_dist_index(active_pos.translation, &board_positions)
        {
            min_dist_index = index;
            found = true;
        }

        // Place card
        if mouse.just_pressed(MouseButton::Left)
            && found
            && let Ok((entity, _)) = active_card
        {
            commands.entity(entity).despawn(); // Remove current active card

            if let Err(e) = com
                .com_layer
                .send(UiToClient::Guess(min_dist_index as Chronology::GuessType))
            {
                error!("Com error: {}", e);
            }
        }

        // Generate new board positions if active card is too far away
        if !found {
            board_positions = board.generate_board_positions(board.cards.len());
        }

        // Re-organize board
        for (i, entity) in board.cards.iter().enumerate() {
            if let Ok((mut transform, _)) = query.get_mut(*entity) {
                if i < min_dist_index {
                    transform.translation = board_positions[i];
                } else {
                    transform.translation = board_positions[i + 1];
                }
            }
        }
    }

    /// Make card lables follow their assigned card.
    pub fn update_card_labels(
        camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
        mut label: Query<(&CardLabel, &mut Node)>,
        card: Query<&GlobalTransform, With<Card>>,
    ) {
        let Ok((camera, camera_transform)) = camera.single() else {
            error!("Found more than one camera");
            return;
        };

        for (label, mut node) in &mut label {
            if let Ok(card_transform) = card.get(label.target)
                && let Ok(screen_pos) =
                    camera.world_to_viewport(camera_transform, card_transform.translation())
            {
                node.left = Val::Px(screen_pos.x);
                node.top = Val::Px(screen_pos.y);
            }
        }
    }

    /// Updates the text of a [`TextQuad`].
    /// This is invoked by sending a [`UpdateTextQuad`]
    /// message.
    pub fn update_text_quad(
        mut events: MessageReader<UpdateTextQuad>,
        children_query: Query<&Children>,
        quad_query: Query<&TextQuad>,
        mut text_query: Query<&mut Text>,
    ) {
        for event in events.read() {
            if let Ok(children) = children_query.get(event.parent) {
                for child in children {
                    if let Ok(text_quad) = quad_query.get(*child)
                        && let Ok(mut text) = text_query.get_mut(text_quad.text_entity)
                    {
                        *text = Text::new(event.text.clone());
                    }
                }
            }
        }
    }

    /// Freezes all [`LooseCard`]s if a [`FreezeBoard`]
    /// message is received.
    pub fn freeze_board(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut reader: MessageReader<FreezeBoard>,
        mut loose_cards: Query<Entity, With<LooseCard>>,
        sounds: Res<GameSounds>,
    ) {
        for _ in reader.read() {
            for entity in &mut loose_cards {
                commands.entity(entity).remove::<LooseCard>();
                commands.entity(entity).insert(SceneRoot(
                    asset_server.load("meshes/card_frozen.glb#Scene0"),
                ));
            }
            GameSounds::play_freeze(&mut commands, &sounds);
        }
    }

    /// Despawns any [`LooseCard`]s if a
    /// [`ClearLooseCards`] message is received.
    pub fn clear_loose_cards(
        mut commands: Commands,
        mut reader: MessageReader<ClearLooseCards>,
        mut board: ResMut<BoardState>,
        loose_cards: Query<Entity, With<LooseCard>>,
        label: Query<(Entity, &CardLabel)>,
        sounds: Res<GameSounds>,
    ) {
        for _ in reader.read() {
            for loose_card in loose_cards {
                for (label_entity, _) in label.iter().filter(|e| e.1.target == loose_card) {
                    commands.entity(label_entity).despawn();
                }
                board.remove_card(loose_card);
                commands.entity(loose_card).despawn();
            }
            GameSounds::play_clear(&mut commands, &sounds);
        }
    }

    /// Shows the inteactive ui used for freeze
    /// option.
    pub fn enable_interactive_ui(
        mut commands: Commands,
        freeze_base: Query<Entity, With<FreezeOptionBase>>,
        freeze_buttons: Query<Entity, With<FreezeOptionButton>>,
        children_query: Query<&Children>,
    ) {
        for entity in &freeze_base {
            commands.entity(entity).insert(Visibility::Visible);
        }
        for btn in freeze_buttons {
            commands.entity(btn).remove::<Disabled>();
            commands.entity(btn).insert(Visibility::Visible);
            enable_children(&mut commands, &children_query, btn);
        }
    }

    /// Shows the interactive ui with just the
    /// continue button. This is used for the
    /// continue option that's presented if a
    /// incrrect guess is made.
    pub fn enable_interactive_ui_continue(
        mut commands: Commands,
        continue_base: Query<Entity, With<FreezeOptionBase>>,
        continue_buttons: Query<(Entity, &FreezeOptionButton), With<FreezeOptionButton>>,
        disabled_children_query: Query<&Children>,
        children_query: Query<&Children>,
    ) {
        for entity in &continue_base {
            commands.entity(entity).insert(Visibility::Visible);
        }
        for (btn, option) in continue_buttons {
            if *option == FreezeOptionButton::Continue {
                commands.entity(btn).remove::<Disabled>();
                commands.entity(btn).insert(Visibility::Visible);
                enable_children(&mut commands, &disabled_children_query, btn);
            } else {
                commands.entity(btn).insert(Visibility::Hidden);
                disable_children(&mut commands, &children_query, btn);
            }
        }
    }

    /// Hides the interactive ui. And disables any
    /// interaction with the buttons.
    pub fn disable_interactive_ui(
        mut commands: Commands,
        freeze_base: Query<Entity, With<FreezeOptionBase>>,
        freeze_buttons: Query<Entity, With<FreezeOptionButton>>,
        children_query: Query<&Children>,
    ) {
        for entity in &freeze_base {
            commands.entity(entity).insert(Visibility::Hidden);
        }
        for btn in freeze_buttons {
            commands.entity(btn).insert(Visibility::Hidden);
            disable_children(&mut commands, &children_query, btn);
        }
    }

    /// Hides all children of a entity. Note that
    /// this function is not recursive.
    fn disable_children(
        commands: &mut Commands,
        children_query: &Query<&Children>,
        entity: Entity,
    ) {
        if let Ok(children) = children_query.get(entity) {
            for child in children {
                commands.entity(*child).insert(Visibility::Hidden);
            }
        }
    }

    /// Shows all children of a entity. Note that
    /// this function is not recursive.
    fn enable_children(commands: &mut Commands, children_query: &Query<&Children>, entity: Entity) {
        if let Ok(children) = children_query.get(entity) {
            for child in children {
                commands.entity(*child).insert(Visibility::Visible);
            }
        }
    }

    /// Set board with multiple cards. This is
    /// used when the player reconnects and is
    /// invoked by receiving a [`SetBoard`]
    /// message.
    pub fn set_board(
        mut board_reader: MessageReader<SetBoard>,
        mut card_writer: MessageWriter<NewPlacedCard>,
    ) {
        for msg in board_reader.read() {
            for wrapped in msg.events.clone() {
                card_writer.write(NewPlacedCard {
                    year: wrapped.event.year,
                    frozen: wrapped.frozen,
                });
            }
        }
    }

    /// Observs if a button with [`FreezeOptionButton`]
    /// component is pressed, and if so takes
    /// action accordinng the [`GameState`].
    pub fn freeze_button_observer(
        event: On<Pointer<Press>>,
        buttons: Query<&FreezeOptionButton>,
        com: ResMut<CommunicationLayer>,
        mut writer: MessageWriter<FreezeBoard>,
        state: Res<State<GameState>>,
    ) {
        if let Ok(button) = buttons.get(event.entity) {
            match button {
                FreezeOptionButton::Continue => {
                    if *state == GameState::FreezeOption
                        && let Err(e) = com.com_layer.send(UiToClient::Freeze(false))
                    {
                        error!("Com error: {}", e);
                    } else if *state == GameState::ContinueOption
                        && let Err(e) = com.com_layer.send(UiToClient::Continue)
                    {
                        error!("Com error: {}", e);
                    }
                }
                FreezeOptionButton::Freeze => {
                    if let Err(e) = com.com_layer.send(UiToClient::Freeze(true)) {
                        error!("Com error: {}", e);
                    }
                    writer.write(FreezeBoard {});
                }
            }
        }
    }

    /// Mutes the audio in the game if the `M`
    /// button is pressed.
    pub fn mute(
        keyboard_input: Res<ButtonInput<KeyCode>>,
        mut audio_controller: Query<&mut AudioSink, With<AudioControll>>,
    ) {
        let Ok(mut sink) = audio_controller.single_mut() else {
            return;
        };

        if keyboard_input.just_pressed(KeyCode::KeyM) {
            sink.toggle_mute();
        }
    }

    pub fn remove_all_cards(
        mut commands: Commands,
        cards: Query<Entity, With<Card>>,
        label: Query<(Entity, &CardLabel)>,
        mut board: ResMut<BoardState>,
    ) {
        for card in cards {
            for (label_entity, _) in label.iter().filter(|e| e.1.target == card) {
                commands.entity(label_entity).despawn();
            }
            board.remove_card(card);
            commands.entity(card).despawn();
        }
    }
}
