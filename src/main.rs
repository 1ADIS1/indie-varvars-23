use bevy::prelude::*;
use parry2d::{
    math::Isometry,
    query::contact,
    shape::{Ball, Shape},
};

pub const PLAYER_MOVEMENT_SPEED: f32 = 200.;

pub const PLANET_SIZE: Vec2 = Vec2::new(400., 400.);
pub const PLANET_ROTATION_SPEED: f32 = 1.;
pub const PLANET_SHRINK_SPEED: f32 = 50.; // b: 15.
pub const PLANET_SHRINK_LIMIT: Vec2 = Vec2::new(80., 80.);

pub const PLAYER_JUMP_STRENGTH: f32 = 200.;
pub const GRAVITY_STRENGTH: f32 = 150.;

pub const OBSTACLE_SIZE: Vec2 = Vec2::new(48., 48.);
pub const OBSTACLE_MOVEMENT_SPEED: f32 = 100.;

#[derive(Component)]
struct Player {
    pub is_jumping: bool,
}

#[derive(Component)]
struct Obstacle;

#[derive(Component)]
struct Planet;

/// Abstraction of the parry2d shapes to store in the component.
#[derive(Component, Clone)]
pub struct Collider {
    pub shape: Ball,
}

#[derive(Event)]
pub struct PlanetSpawnEvent {
    last_planet_position: Vec3,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<PlanetSpawnEvent>()
        .add_systems(Startup, (spawn_2d_camera, start_game, spawn_player))
        .add_systems(
            Update,
            (
                spawn_planet,
                spawn_obstacles.after(spawn_planet),
                rotate_planets,
                shrink_current_planet,
                player_jump,
                apply_gravity_on_player,
                show_gizmos,
                check_player_planet_collisions,
            ),
        )
        .run();
}

fn spawn_2d_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle { ..default() });
}

fn start_game(mut planet_spawn_event_writer: EventWriter<PlanetSpawnEvent>) {
    planet_spawn_event_writer.send(PlanetSpawnEvent {
        last_planet_position: Vec3::new(0., PLANET_SIZE.y * 2., 0.),
    });
}

fn spawn_planet(
    mut planet_spawn_event_reader: EventReader<PlanetSpawnEvent>,
    mut commands: Commands,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    asset_server: Res<AssetServer>,
) {
    for planet_spawn_event in planet_spawn_event_reader.iter() {
        let mut new_planet_position = planet_spawn_event.last_planet_position;
        new_planet_position.y -= PLANET_SIZE.y * 2.;

        // Create planet collider
        let planet_radius = PLANET_SIZE.y / 2.0;
        let collider_shape = Ball::new(planet_radius);

        commands.spawn((
            SpriteBundle {
                transform: Transform::from_translation(new_planet_position),
                texture: asset_server.load("art/ball.png"),
                sprite: Sprite {
                    custom_size: Some(PLANET_SIZE),
                    ..default()
                },
                ..default()
            },
            Planet,
            Collider {
                shape: collider_shape,
            },
        ));

        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            camera_transform.translation = new_planet_position;
        }

        println!("Planet has spawned!");
    }
}

fn rotate_planets(mut planets_query: Query<&mut Transform, With<Planet>>, time: Res<Time>) {
    for mut planet_transform in planets_query.iter_mut() {
        planet_transform.rotate_z(-PLANET_ROTATION_SPEED * time.delta_seconds());
    }
}

fn shrink_current_planet(
    mut commands: Commands,
    mut planets_query: Query<(&mut Sprite, Entity, &mut Collider, &Transform), With<Planet>>,
    mut planet_spawn_event_writer: EventWriter<PlanetSpawnEvent>,
    time: Res<Time>,
) {
    for (mut planet_sprite, planet_entity, mut collider, transform) in planets_query.iter_mut() {
        let new_planet_size =
            planet_sprite.custom_size.unwrap() - PLANET_SHRINK_SPEED * time.delta_seconds();
        collider.shape.radius -= PLANET_SHRINK_SPEED / 2.0 * time.delta_seconds();

        planet_sprite.custom_size = Some(new_planet_size);

        if new_planet_size.distance(PLANET_SHRINK_LIMIT) < 1. {
            planet_spawn_event_writer.send(PlanetSpawnEvent {
                last_planet_position: transform.translation,
            });
            commands.entity(planet_entity).despawn();
        }
    }
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let collider_shape = Ball::new(32.);

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("art/ball.png"),
            sprite: Sprite {
                color: Color::GREEN,
                ..default()
            },
            transform: Transform::from_xyz(0., 256., 10.),
            ..default()
        },
        Player { is_jumping: false },
        Collider {
            shape: collider_shape,
        },
    ));
}

fn player_jump(
    mut player_query: Query<(&mut Transform, &mut Player)>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if let Ok((mut player_transform, mut player_struct)) = player_query.get_single_mut() {
        if keyboard_input.pressed(KeyCode::Space) {
            player_transform.translation.y += PLAYER_JUMP_STRENGTH * time.delta_seconds();
            // player_struct.is_jumping = true;
        }
    }
}

fn apply_gravity_on_player(mut player_query: Query<&mut Transform, With<Player>>, time: Res<Time>) {
    if let Ok(mut player_transform) = player_query.get_single_mut() {
        player_transform.translation.y -= GRAVITY_STRENGTH * time.delta_seconds();
    }
}

/// When pressing G - renders all gizmos.
pub fn show_gizmos(
    mut gizmos: Gizmos,
    collider_query: Query<(&Transform, &Collider)>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.pressed(KeyCode::G) {
        for (transform, collider) in collider_query.iter() {
            let collider_position = transform.translation;
            gizmos.circle_2d(
                Vec2::new(collider_position.x, collider_position.y),
                collider.shape.radius,
                Color::RED,
            );
        }
    }
}

fn check_player_planet_collisions(
    mut player_query: Query<(&Collider, &mut Transform), (With<Player>, Without<Planet>)>,
    planet_query: Query<(&Collider, &Transform), With<Planet>>,
) {
    for (player_collider, mut player_transform) in player_query.iter_mut() {
        for (planet_collider, planet_transform) in planet_query.iter() {
            let mut player_translation = player_transform.translation;

            let actor_isometry = Isometry::translation(
                player_transform.translation.x,
                player_transform.translation.y,
            );
            let tile_isometry = Isometry::translation(
                planet_transform.translation.x,
                planet_transform.translation.y,
            );

            let actor_shape = player_collider.shape.clone_box();
            let tile_shape = planet_collider.shape.clone_box();

            // Distance between objects to collide
            let distance = 0.0;
            let collision = contact(
                &actor_isometry,
                &*actor_shape,
                &tile_isometry,
                &*tile_shape,
                distance,
            )
            .unwrap();

            // If objects collided
            if let Some(contact) = collision {
                let normal = contact.normal1.into_inner();

                player_translation.x += contact.dist * normal.x;
                player_translation.y += contact.dist * normal.y;
            }

            player_transform.translation = player_translation;
        }
    }
}

fn spawn_obstacles(
    mut planet_spawn_event_reader: EventReader<PlanetSpawnEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for planet_spawn_event in planet_spawn_event_reader.iter() {
        let obstacle_position = Vec3::new(0., -(PLANET_SIZE.y + OBSTACLE_SIZE.y) / 2., 0.);

        commands.spawn((
            SpriteBundle {
                transform: Transform::from_translation(obstacle_position),
                texture: asset_server.load("art/ball.png"),
                sprite: Sprite {
                    color: Color::ORANGE,
                    custom_size: Some(OBSTACLE_SIZE),
                    ..default()
                },
                ..default()
            },
            Collider {
                shape: Ball::new(OBSTACLE_SIZE.y / 2.),
            },
            Obstacle,
        ));

        println!("Obstacles have spawned!");
    }
}

// /// Handles the player movement each frame by updating it's **transform** component.
// fn move_player(
//     mut player_query: Query<&mut Transform, With<Player>>,
//     keyboard_input: Res<Input<KeyCode>>,
//     time: Res<Time>,
// ) {
//     if let Ok(mut player_transform) = player_query.get_single_mut() {
//         let mut direction = Vec3::ZERO;

//         if keyboard_input.pressed(KeyCode::A) {
//             direction.x -= 1.;
//         }
//         if keyboard_input.pressed(KeyCode::D) {
//             direction.x += 1.;
//         }
//         if keyboard_input.pressed(KeyCode::W) {
//             direction.y += 1.;
//         }
//         if keyboard_input.pressed(KeyCode::S) {
//             direction.y -= 1.;
//         }

//         let direction = direction.normalize_or_zero();
//         player_transform.translation += direction * PLAYER_MOVEMENT_SPEED * time.delta_seconds();
//     }
// }
