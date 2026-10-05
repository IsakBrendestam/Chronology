use bevy::prelude::*;

use crate::game_definitions::*;

use bevy::{
    camera::RenderTarget,
    picking::{
        backend::ray::RayMap,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    window::{PrimaryWindow, WindowEvent},
};

const POINTER_ID: PointerId = PointerId::Mouse;

/// Formatts text to be displayed in the game-display.
/// Where the upper and lower text will be divided
/// by a line.
pub fn format_display_text(upper_text: &str, lower_text: &str) -> String {
    let max_chars = 21;
    let category_wraped = textwrap::wrap(upper_text, max_chars).join("\n");
    let text_wraped = textwrap::wrap(lower_text, max_chars).join("\n");

    format!(
        "{}\n{}\n{}",
        category_wraped,
        "-".repeat(max_chars),
        text_wraped
    )
}

/// This code origionates from an example found
/// on the bevy webbpage:
/// https://bevy.org/examples/ui-user-interface/render-ui-to-texture/
///
/// However the code has been modified.
#[allow(clippy::too_many_arguments)]
pub fn drive_diegetic_pointer(
    mut cursor_last: Local<Vec2>,
    mut raycast: MeshRayCast,
    rays: Res<RayMap>,
    quads: Query<&Mesh3d, With<InteractiveUiQuad>>,
    ui_camera: Query<&RenderTarget, With<InteractiveUiCamera>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut window_events: MessageReader<WindowEvent>,
    mut pointer_inputs: MessageWriter<PointerInput>,
) -> Result {
    for render_target in &ui_camera {
        let Some(target) = render_target.normalize(primary_window.single().ok()) else {
            continue;
        };

        let Ok(target_info) =
            target.get_render_target_info(windows, &images, &manual_texture_views)
        else {
            continue;
        };

        let size = target_info.physical_size.as_vec2();

        let settings = MeshRayCastSettings {
            visibility: RayCastVisibility::Any,
            filter: &|e| quads.contains(e),
            early_exit_test: &|_| false,
        };

        let mut delta = Vec2::ZERO;

        for (_id, ray) in rays.iter() {
            for (_quad, hit) in raycast.cast_ray(*ray, &settings) {
                if let Some(uv) = hit.uv {
                    let pos = size * uv;
                    if pos != *cursor_last {
                        delta = pos - *cursor_last;
                        *cursor_last = pos;
                    }
                }
            }
        }

        for event in window_events.read() {
            if let WindowEvent::MouseButtonInput(_) = event {
                pointer_inputs.write(PointerInput::new(
                    POINTER_ID,
                    Location {
                        target: target.clone(),
                        position: *cursor_last,
                    },
                    PointerAction::Move { delta }, // For some reason this will also send button presses
                ));
            }
        }
    }

    Ok(())
}

/// Moves a object with [`MoveComp`]. This is used
/// for debug purposes.
pub fn move_system_debug(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<MoveComp>>,
    time: Res<Time>,
) {
    let move_speed = 3.0;
    let rot_speed = 2.0;
    let scale_speed = 1.0;

    let dt = time.delta_secs();

    let mut movement = Vec3::ZERO;
    let mut rot = Vec3::ZERO;
    let mut scale = Vec3::ZERO;

    // Movement
    if keyboard_input.pressed(KeyCode::KeyW) {
        movement.x -= move_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        movement.x += move_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        movement.z += move_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        movement.z -= move_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::Space) {
        movement.y += move_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::ControlLeft) {
        movement.y -= move_speed * dt;
    }

    // Rotation (arrow keys)
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        rot.x += rot_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        rot.x -= rot_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        rot.y += rot_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        rot.y -= rot_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::ShiftRight) {
        rot.z += rot_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::ControlRight) {
        rot.z -= rot_speed * dt;
    }

    if keyboard_input.pressed(KeyCode::KeyI) {
        scale.x -= scale_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyK) {
        scale.x += scale_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyJ) {
        scale.y -= scale_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyL) {
        scale.y += scale_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyU) {
        scale.z -= scale_speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyO) {
        scale.z += scale_speed * dt;
    }

    if movement != Vec3::ZERO || rot != Vec3::ZERO || scale != Vec3::ZERO {
        for mut transform in &mut query {
            transform.translation += movement;

            transform.scale += scale;

            transform.rotate_x(rot.x);
            transform.rotate_y(rot.y);
            transform.rotate_z(rot.z);

            println!("Transform: {:?}", transform);
        }
    }
}

/// Projects the mouse position to a horizontal
/// plane a y = 5, that represents the board.
pub fn get_mouse_board_position(
    window: &Single<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<Vec3> {
    let Ok((camera, camera_transform)) = camera.single() else {
        error!("Found more than one camera");
        return None;
    };

    if let Some(cursor_pos) = window.cursor_position()
        && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos)
    {
        // Intersect with plane at y = 5.0
        let plane_y = 5.0;

        let distance = (plane_y - ray.origin.y) / ray.direction.y;
        let world_position = ray.origin + ray.direction * distance;
        return Some(world_position);
    }

    None
}
