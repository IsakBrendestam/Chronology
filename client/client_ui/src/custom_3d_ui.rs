//! Collection of functions that provides
//! functionality for creating Becy-UI
//! elements in 3D space.
//!
//! This is done by rendereing the UI
//! to a texture and the applying this
//! texture to 3D onjects.

pub mod ui_3d {
    use core::f32;

    use bevy::asset::RenderAssetUsages;
    use bevy::camera::RenderTarget;
    use bevy::prelude::*;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

    use crate::game_definitions::*;

    /// Data for creating a 3D quad with ui
    /// elemements.
    ///
    /// parent: The parent eneity that the quad
    ///   will be added to.
    ///
    /// font_size: size of the text
    ///
    /// transform: local transform for the quad
    ///
    /// text: text that will be rendered
    ///
    /// font: fond that will be used for the text
    pub struct TextQuadData<'a> {
        pub parent: Entity,
        pub font_size: f32,
        pub transform: Transform,
        pub text: &'a str,
        pub font: Option<Handle<Font>>,
    }

    /// Renderes UI text elemetns to a texture
    /// and applies this texture to a quad in
    /// 3D space.
    pub fn add_text_quad(
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        images: &mut Assets<Image>,
        data: TextQuadData,
    ) {
        // Creating render texture
        let size = Extent3d {
            width: 512,
            height: 512,
            ..default()
        };

        let image_handle = ui_image_handle(size, images);

        let cam = ui_camera(commands, image_handle.clone());

        // Create UI elements
        let text_entity = text_ui(commands, cam, data.font, data.font_size, data.text);

        // Create quad
        let (quad, material) = ui_quad(size, meshes, materials, image_handle);

        // Attach quad to entity
        let quad_entity = commands
            .spawn((
                Name::new("TextQuad"),
                Mesh3d(quad),
                MeshMaterial3d(material),
                data.transform,
                TextQuad { text_entity },
            ))
            .id();

        commands.entity(data.parent).add_child(quad_entity);
    }

    /// Renderes UI two UI buttons to a texture
    /// and applies this texture to a quad in
    /// 3D space. The buttons will have the
    /// [`FreezeOptionButton`] component, and
    /// can thus be queried with this.
    pub fn add_freeze_option_quad(
        commands: &mut Commands,
        parent: Entity,
        transform: Transform,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        images: &mut Assets<Image>,
    ) {
        // Creating render texture
        let size = Extent3d {
            width: 512,
            height: 512,
            ..default()
        };

        let image_handle = ui_image_handle(size, images);

        let cam = ui_camera(commands, image_handle.clone());
        commands.entity(cam).insert(InteractiveUiCamera);

        freeze_option_ui(commands, cam);

        // Create quad
        let (quad, material) = ui_quad(size, meshes, materials, image_handle);

        // Attach quad to entity
        commands.entity(parent).with_children(|p| {
            p.spawn((
                Name::new("FreezeOptionQuad"),
                Mesh3d(quad),
                MeshMaterial3d(material),
                transform,
                InteractiveUiQuad,
                Visibility::default(),
                InheritedVisibility::default(),
                Pickable {
                    should_block_lower: true,
                    is_hoverable: true,
                },
            ));
        });
    }

    /// Creates a ui image handle
    fn ui_image_handle(size: Extent3d, images: &mut Assets<Image>) -> Handle<Image> {
        let mut image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 0], // transparent background
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::default(),
        );

        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;

        images.add(image)
    }

    /// Creates a ui (2D) camera.
    fn ui_camera(commands: &mut Commands, image_handle: Handle<Image>) -> Entity {
        commands
            .spawn((
                Name::new("UiCamera"),
                Camera2d,
                Camera {
                    order: -1,
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    ..default()
                },
                RenderTarget::Image(image_handle.clone().into()),
            ))
            .id()
    }

    /// Creates a 3D quad with material
    fn ui_quad(
        size: Extent3d,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        image_handle: Handle<Image>,
    ) -> (Handle<Mesh>, Handle<StandardMaterial>) {
        let quad = meshes.add(Rectangle::new(size.width as f32 / size.height as f32, 1.0));

        let material = materials.add(StandardMaterial {
            base_color_texture: Some(image_handle),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        (quad, material)
    }

    /// Creats a ui entity with text.
    fn text_ui(
        commands: &mut Commands,
        cam: Entity,
        font: Option<Handle<Font>>,
        font_size: f32,
        text: &str,
    ) -> Entity {
        let max_width = 512.0;
        let estimated_width = text.len() as f32 * font_size * 0.6;
        let scale = (max_width / estimated_width).min(1.0);

        let fitted_font_size = font_size * scale;

        let mut text_font = TextFont {
            font_size: fitted_font_size,
            ..default()
        };

        if let Some(font) = font {
            text_font.font = font;
        }

        let mut text_entity = Entity::PLACEHOLDER;

        commands
            .spawn((
                Name::new("TextEntityParent"),
                Node {
                    width: percent(100),
                    height: percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                UiTargetCamera(cam),
            ))
            .with_children(|p| {
                text_entity = p
                    .spawn((
                        Name::new("TextEntity"),
                        Text::new(text),
                        text_font,
                        TextColor(Color::WHITE),
                        Node {
                            width: percent(100),
                            max_width: percent(100),
                            align_self: AlignSelf::FlexStart,
                            ..default()
                        },
                        TextLayout::new_with_justify(Justify::Center),
                    ))
                    .id();
            });

        text_entity
    }

    /// Creates a ui entity with a continue and
    /// freeze button
    fn freeze_option_ui(commands: &mut Commands, cam: Entity) {
        let ui_root = Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(20.0),
            ..default()
        };

        let root = commands
            .spawn((
                Name::new("FreezeRoot"),
                ui_root,
                UiTargetCamera(cam),
                BackgroundColor(Color::NONE),
            ))
            .id();

        // Spawn child entities
        let continue_btn = commands
            .spawn((
                Name::new("ContinueButton"),
                button_node("Continue"),
                FreezeOptionButton::Continue,
            ))
            .id();
        let freeze_btn = commands
            .spawn((
                Name::new("FreezeButton"),
                button_node("Freeze"),
                FreezeOptionButton::Freeze,
            ))
            .id();

        commands
            .entity(root)
            .add_children(&[continue_btn, freeze_btn]);
    }

    /// Crates a node for a button.
    fn button_node(label: &str) -> impl Bundle {
        (
            Node {
                width: px(180),
                height: px(65),
                border: UiRect::all(px(5)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BackgroundColor(Color::BLACK),
            children![(
                Name::new("LabelNode"),
                Text::new(label),
                TextFont {
                    font_size: 33.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
                Visibility::Inherited,
            )],
            Pickable {
                should_block_lower: true,
                is_hoverable: true,
            },
        )
    }
}
