use std::time::Duration;

use bevy::{math::{vec2, vec3}, prelude::*};
use rand::Rng;
use bevy_editor_pls::prelude::*;

const SCREEN_NORMAL_SIZE: Vec2 = vec2(1280.0, 720.0);
const PLAYER_SIZE: Vec2 = vec2(10.0, 20.0);
const PLAYER_X: f32 = 3.0 / 4.0;
const JUMP_STRENGTH: f32 = 200.0;
const DROP_STRENGTH: f32 = 100.0;
const GRAVITY_LEVEL: f32 = 100.0;
const PLATFORM_HIGHT: f32 = 10.0;
const PLATFORM_MIN_WIDTH: u16 = 200;
const PLATFORM_MAX_WIDTH: u16 = 400;
const PLATFROM_TOP_Y: f32 = 100.0;
const PLATFROM_BOTTOM_Y: f32 = -100.0;
const PLATFROM_SPAWN_RATE_SECS: f32 = 1.0;
const STARTING_GAME_SPEED: f32 = 200.0;
const JUMP_BUFFER: f32 = 0.1;

fn main() {
    App::new()
        .insert_resource(GameSpeed(STARTING_GAME_SPEED))
        .insert_resource(PlatformSpawnTimer(Timer::new(Duration::from_secs_f32(PLATFROM_SPAWN_RATE_SECS), TimerMode::Repeating)))
        .add_plugins(DefaultPlugins)
        .add_plugins(EditorPlugin::default())
        .add_systems(Startup, setup_system)
        .add_systems(Update, (input_system, velocity_system, spawner_system, platform_moving_system, player_system))
        .register_type::<Velocity>()
        .register_type::<PlayerStatusManager>()
        .run();
}

fn setup_system(
    mut commands: Commands,
    window: Query<&Window>
) {
    let window = window.single();
    let window_width = window.resolution.width();
    let _window_height = window.resolution.height();

    // Spawn camera
    commands.spawn(Camera2dBundle::default());

    // Spawn player
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: vec3((window_width / 2.0) * -PLAYER_X, 0.0, 1.0),
                ..default()
            },
            sprite: Sprite {
                color: Color::rgb(0.0, 1.0, 1.0),
                custom_size: Some(PLAYER_SIZE),
                ..default()
            }, 
            ..default()
        },
        Velocity {
            y: 0.0
        },
        Player,
        PlayerStatusManager {
            jump_buffer: 10.0,
            space_shipping: true,
            hanging: false
        },
        Name::new("Player")
    ));
}

fn velocity_system(
    mut object_query: Query<(&mut Transform, &mut Velocity, Option<&mut PlayerStatusManager>, &Sprite)>,
    platform_query: Query<(&Transform, &Sprite), (With<Platform>, Without<Velocity>)>,
    // mut debug_query: Query<&mut Transform, With<DebugTarget>>,
    window: Query<&Window>,
    time: Res<Time>
) {
    let window = get_window_size(window.single());

    for (mut object_transform, mut object_velocity, mut player_status_manager, object_sprite) in &mut object_query {
        let copy_of_status_manager = player_status_manager.as_deref();
        if let status = copy_of_status_manager.unwrap().clone() {
            let shipping: bool = status.space_shipping;
            if shipping {
                continue;
            }
        }


        let p_y = object_transform.translation.y;

        object_velocity.y -= GRAVITY_LEVEL * time.delta_seconds();
        object_transform.translation.y += (object_velocity.y * time.delta_seconds() / SCREEN_NORMAL_SIZE.y) * window.y;
        object_velocity.y -= GRAVITY_LEVEL * time.delta_seconds();

        let object_size = object_sprite.custom_size.unwrap_or_default();


        // Set the hanging value to false before going through the platforms
        if let Some(player_status_manager) = player_status_manager.as_deref_mut() {
            player_status_manager.hanging = false;
        }


        // Check to see if the object is on a platform
        for (platform_transform, platfrom_sprite) in &platform_query {
            let platform_size: Vec2 = platfrom_sprite.custom_size.unwrap_or_default();

            // Check if the object is interacting above or below the platform
            let bottom_of_object = object_transform.translation.y - object_size.y / 2.0;
            let p_bottom_of_object = p_y - object_size.y / 2.0;
            let top_of_object = object_transform.translation.y + object_size.y / 2.0;
            let p_top_of_object = p_y + object_size.y / 2.0;
            let top_of_platform = platform_transform.translation.y + platform_size.y / 2.0;
            let bottom_of_platform = platform_transform.translation.y - platform_size.y / 2.0;
            let object_above_platform = bottom_of_object > top_of_platform;
            let p_object_above_platform = p_bottom_of_object >= top_of_platform;
            let object_below_platform = top_of_object < bottom_of_platform;
            let p_object_below_platform = p_top_of_object <= bottom_of_platform;

            // Check if the object is aligned with the platform
            let object_right = object_transform.translation.x + object_size.x / 2.0;
            let object_left = object_right - object_size.x;
            let platform_right = platform_transform.translation.x + platform_size.x / 2.0;
            let platform_left = platform_right - platform_size.x;
            let over_right = object_left > platform_right;
            let over_left = object_right < platform_left;
            let aligned_with_platform = !over_left && !over_right;

            let on_platform = !object_above_platform && p_object_above_platform && aligned_with_platform;
            let bumping_bottom_of_platform = !object_below_platform && p_object_below_platform && aligned_with_platform;

            if on_platform {
                object_transform.translation.y = top_of_platform + object_size.y / 2.0;
                object_velocity.y = 0.0;

                // If the object has the PlayerStatusManager than allow the jumping to happen
                if let Some(player_status_manager) = player_status_manager.as_deref_mut() {
                    player_status_manager.jump_buffer = 0.0;
                }
            }

            if bumping_bottom_of_platform {
                object_transform.translation.y = bottom_of_platform - object_size.y / 2.0;
                object_velocity.y = 100.0;

                if let Some(player_status_manager) = player_status_manager.as_deref_mut() {
                    player_status_manager.hanging = true;
                }
            }
        }
    }
}

fn input_system(
    mut query: Query<(&mut Velocity, &mut PlayerStatusManager), With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>
) {
    for (mut velocity, mut player_status_manager) in &mut query {
        let jump_button_pressed = input.just_pressed(KeyCode::Space);
        if jump_button_pressed && (player_status_manager.jump_buffer < JUMP_BUFFER || player_status_manager.space_shipping) {
            velocity.y = JUMP_STRENGTH;
            player_status_manager.jump_buffer = 100.0;
            player_status_manager.space_shipping = false;
        } else if player_status_manager.hanging && jump_button_pressed {
            velocity.y = -DROP_STRENGTH;
            player_status_manager.jump_buffer = 100.0;
        }

        player_status_manager.jump_buffer += time.delta_seconds();
    }
}

fn player_system(
    mut query: Query<(&mut Transform, &mut Sprite), With<Player>>,
    window: Query<&Window>
) {
    let window = get_window_size(window.single());

    for (mut transform, mut sprite) in &mut query {
        transform.translation.x = (window.x / 2.0) * -PLAYER_X;
        sprite.custom_size = Some((PLAYER_SIZE / SCREEN_NORMAL_SIZE.y) * window.y);
    }
}

fn spawner_system(
    window: Query<&Window>,
    time: Res<Time>,
    mut platfrom_spawn_timer: ResMut<PlatformSpawnTimer>,
    mut commands: Commands
) {
    let mut rng = rand::thread_rng();
    let window = get_window_size(window.single());

    platfrom_spawn_timer.0.tick(time.delta());

    if platfrom_spawn_timer.0.just_finished() {
        // Spawn new platform
        let width = rng.gen_range(PLATFORM_MIN_WIDTH..=PLATFORM_MAX_WIDTH);
        let starting_x = window.x / 2.0 + (width as f32 / SCREEN_NORMAL_SIZE.y) * window.y;
        let starting_y = rng.gen_range(PLATFROM_BOTTOM_Y..=PLATFROM_TOP_Y);
        commands.spawn((
            SpriteBundle {
                transform: Transform {
                    translation: vec3(starting_x, starting_y, 0.0),
                    ..default()
                },
                sprite: Sprite {
                    custom_size: Some((vec2(width as f32, PLATFORM_HIGHT) / SCREEN_NORMAL_SIZE.y) * window.y),
                    ..default()
                },
                ..default()
            },
            Platform
        ));
    }
}

fn platform_moving_system(
    mut query: Query<(Entity, &mut Transform, &Sprite), With<Platform>>,
    mut commands: Commands,
    game_speed: Res<GameSpeed>,
    window: Query<&Window>,
    time: Res<Time>
) {
    let window = get_window_size(window.single());


    for (entity, mut transform, sprite) in &mut query {
        transform.translation.x -= (game_speed.0 / SCREEN_NORMAL_SIZE.y) * window.y * time.delta_seconds();

        let size = sprite.custom_size.unwrap_or_default();
        let width = size.x;
        let point_of_no_return = -window.x / 2.0 - (width as f32 / SCREEN_NORMAL_SIZE.y) * window.y;
        if transform.translation.x < point_of_no_return {
            commands.entity(entity).despawn();
        }
    }
}

fn get_window_size(window: &Window) -> Vec2 {
    let window = vec2(window.resolution.width(), window.resolution.height());
    return window;
}


#[derive(Component)]
struct Player;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Velocity {
    y: f32
}

#[derive(Component)]
struct Platform;

#[derive(Component)]
struct DebugTarget;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PlayerStatusManager {
    jump_buffer: f32,
    space_shipping: bool,
    hanging: bool
}

#[derive(Resource)]
struct PlatformSpawnTimer(Timer);

#[derive(Resource)]
struct GameSpeed(f32);