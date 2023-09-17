mod ui;

use std::{f32::consts::*, time::Duration};

use bevy::{asset::LoadState, prelude::*, window::PresentMode};
use bevy_tweening::{lens::TransformPositionLens, *};
use parry2d::{
    math::Isometry,
    query::contact,
    shape::{Ball, Shape},
};
use rand::Rng;
use ui::{ReplayButton, UIPlugin};

pub const PLAYER_MOVEMENT_SPEED: f32 = 200.;
pub const PLAYER_JUMP_STRENGTH: f32 = 450.;
pub const GRAVITY_STRENGTH: f32 = -27.43;
pub const PLAYER_FALL_ACCELERATION: f32 = -3000.;

pub const PLANET_SIZE: Vec2 = Vec2::new(715., 715.);
pub const PLANET_ROTATION_SPEED: f32 = 1.;
pub const PLANET_SHRINK_SPEED: f32 = 50.; // b: 15.
pub const PLANET_SHRINK_LIMIT: Vec2 = Vec2::new(200., 200.);

pub const PLANET_FACE_SIZE: Vec2 = Vec2::new(715., 715.);
pub const PLANET_FACE_NORMAL_THRESHOLD: f32 = 250.;
pub const PLANET_FACE_BAD_THRESHOLD: f32 = 175.;

pub const OBSTACLE_SIZE: Vec2 = Vec2::new(48., 48.);
pub const OBSTACLE_MOVEMENT_SPEED: f32 = 2.;
// 20 degrees - 45 degrees
pub const OBSTACLE_CLOSE_GAP_RANGE: (f32, f32) = (0., 0.261799);
// 40 degrees - 80 degrees
pub const OBSTACLE_LONG_GAP_RANGE: (f32, f32) = (0.698132, 1.39626);
// 180 degrees
pub const OBSTACLE_MAX_ANGLE_GENERATION: f32 = PI;
// 45 degrees
pub const OBSTACLE_MIN_ANGLE_GENERATION: f32 = FRAC_PI_4;

pub const PLAYER_START_POSITION: Vec3 = Vec3::new(0., PLANET_SIZE.y, 0.);

/// Resource for tracking loading assets.
#[derive(Resource, Default)]
pub struct AssetsLoading(Vec<HandleUntyped>);

/// Used to load assets when the game starts.
#[derive(States, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub enum LoadingState {
    #[default]
    Planet,
    None,
}

#[derive(States, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    #[default]
    Playing,
    GameOver,
}

#[derive(Component)]
struct Player {
    pub is_grounded: bool,
    velocity: f32,
}

#[derive(Component)]
struct Obstacle {
    angle: f32,
}

#[derive(Component)]
struct Planet {
    variant: PlanetVariant,
    is_playing: bool,
    obstacles: Vec<Entity>,
    radius: f32,
}

#[derive(Clone, Copy)]
pub enum PlanetVariant {
    Earth = 0,
    Venus,
    Mars,
    Mercury,
}

impl PlanetVariant {
    fn next(self) -> PlanetVariant {
        match self {
            PlanetVariant::Earth => PlanetVariant::Venus,
            PlanetVariant::Venus => PlanetVariant::Mars,
            PlanetVariant::Mars => PlanetVariant::Mercury,
            PlanetVariant::Mercury => PlanetVariant::Earth,
        }
    }
}

#[derive(Component)]
struct PlanetFace {
    face: PlanetFaceState,
}

/// Abstraction of the parry2d shapes to store in the component.
#[derive(Component, Clone)]
pub struct Collider {
    pub shape: Ball,
}

#[derive(Event)]
pub struct PlanetSpawnEvent {
    planet_variant_to_spawn: PlanetVariant,
    last_planet_position: Vec3,
}

pub enum PlanetFaceState {
    Good,
    Normal,
    Bad,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Indie Varvar's 2023".into(),
                resolution: (840., 750.).into(),
                present_mode: PresentMode::AutoVsync,
                // mode: WindowMode::BorderlessFullscreen,
                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TweeningPlugin)
        .add_plugins(UIPlugin)
        .add_event::<PlanetSpawnEvent>()
        .add_state::<LoadingState>()
        .add_state::<AppState>()
        .init_resource::<AssetsLoading>()
        .add_systems(Startup, spawn_2d_camera)
        .add_systems(OnEnter(AppState::Playing), (start_game, spawn_player))
        .add_systems(
            Update,
            (
                spawn_planet.after(shrink_current_planet),
                rotate_planets,
                shrink_current_planet,
                player_jump,
                show_gizmos,
                check_player_planet_collisions.after(player_jump),
                move_obstacles_on_planet,
                check_player_obstacle_collisions,
                manage_planet_face,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            check_planets_loading.run_if(in_state(LoadingState::Planet)),
        )
        .add_systems(OnExit(LoadingState::Planet), spawn_obstacles)
        .add_systems(OnEnter(AppState::GameOver), restart_game)
        .run();
}

fn spawn_2d_camera(mut commands: Commands) {
    let tween = Tween::new(
        EaseFunction::QuadraticInOut,
        Duration::from_secs(0),
        TransformPositionLens {
            start: Vec3::ZERO,
            end: Vec3::ZERO,
        },
    );

    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_translation(PLAYER_START_POSITION),
            projection: OrthographicProjection {
                far: 1000.,
                near: -1000.,
                scale: 1.5,
                ..default()
            },
            ..default()
        },
        // Add an Animator component to control and execute the animation.
        Animator::new(tween),
    ));
}

fn start_game(mut planet_spawn_event_writer: EventWriter<PlanetSpawnEvent>) {
    planet_spawn_event_writer.send(PlanetSpawnEvent {
        planet_variant_to_spawn: PlanetVariant::Earth,
        last_planet_position: Vec3::new(0., PLANET_SIZE.y * 2., 0.),
    });
}

// TODO: fix bug when new planet does not have obstacles.
fn restart_game(
    mut commands: Commands,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    despawn_entities: Query<
        Entity,
        (
            Or<(With<Planet>, With<Obstacle>, With<Player>)>,
            (Without<Camera>, Without<ReplayButton>),
        ),
    >,
) {
    println!("Len: {}", despawn_entities.iter().len());
    for entity_to_despawn in despawn_entities.iter() {
        commands.entity(entity_to_despawn).despawn_recursive();
    }

    if let Ok(mut camera_transform) = camera_query.get_single_mut() {
        camera_transform.translation = PLAYER_START_POSITION;
    }
}

fn spawn_planet(
    mut planet_spawn_event_reader: EventReader<PlanetSpawnEvent>,
    mut commands: Commands,
    mut camera_query: Query<(&Transform, &mut Animator<Transform>), With<Camera>>,
    mut loading: ResMut<AssetsLoading>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    for planet_spawn_event in planet_spawn_event_reader.iter() {
        let texture = match planet_spawn_event.planet_variant_to_spawn {
            PlanetVariant::Earth => asset_server.load("art/Earth.png"),
            PlanetVariant::Mars => asset_server.load("art/Mars.png"),
            PlanetVariant::Venus => asset_server.load("art/Venus.png"),
            PlanetVariant::Mercury => asset_server.load("art/Mercury.png"),
        };

        let mut new_planet_position = planet_spawn_event.last_planet_position;
        new_planet_position.y -= PLANET_SIZE.y * 2.;

        // Create planet collider
        let planet_radius = PLANET_SIZE.y / 2.0;
        let collider_shape = Ball::new(planet_radius);

        commands
            .spawn((
                SpriteBundle {
                    transform: Transform::from_translation(new_planet_position),
                    texture: texture.clone(),
                    sprite: Sprite {
                        custom_size: Some(PLANET_SIZE),
                        ..default()
                    },
                    ..default()
                },
                Planet {
                    variant: planet_spawn_event.planet_variant_to_spawn,
                    is_playing: false,
                    obstacles: Vec::new(),
                    radius: PLANET_SIZE.y / 2.,
                },
                Collider {
                    shape: collider_shape,
                },
            ))
            .with_children(|parent| {
                let face_spritesheet = asset_server.load("art/FaceAtlas.png");
                let face_atlas =
                    TextureAtlas::from_grid(face_spritesheet, PLANET_FACE_SIZE, 3, 1, None, None);
                let texture_atlas_handle = texture_atlases.add(face_atlas);

                parent.spawn((
                    SpriteSheetBundle {
                        sprite: TextureAtlasSprite {
                            index: 0,
                            custom_size: Some(PLANET_FACE_SIZE),
                            ..default()
                        },
                        texture_atlas: texture_atlas_handle,
                        transform: Transform::from_xyz(0., 0., 10.),
                        ..default()
                    },
                    PlanetFace {
                        face: PlanetFaceState::Good,
                    },
                ));
            });

        loading.0.push(texture.clone_untyped());

        // Tween camera position
        if let Ok((camera_transform, mut camera_animator)) = camera_query.get_single_mut() {
            // // camera_transform.translation = new_planet_position;
            // Create a single animation (tween) to move an entity.
            let tween = Tween::new(
                // Use a quadratic easing on both endpoints.
                EaseFunction::QuadraticInOut,
                // Animation time (one way only; for ping-pong it takes 2 seconds
                // to come back to start).
                Duration::from_secs_f32(1.2),
                // The lens gives the Animator access to the Transform component,
                // to animate it. It also contains the start and end values associated
                // with the animation ratios 0. and 1.
                TransformPositionLens {
                    start: camera_transform.translation,
                    end: new_planet_position,
                },
            );

            camera_animator.set_tweenable(tween);
        }
    }
}

fn check_planets_loading(
    mut next_loading_state: ResMut<NextState<LoadingState>>,
    mut loading: ResMut<AssetsLoading>,
    asset_server: Res<AssetServer>,
) {
    if asset_server.get_group_load_state(loading.0.iter().map(|handle| handle.id()))
        == LoadState::Loaded
    {
        // all assets are now ready
        next_loading_state.set(LoadingState::None);

        loading.0.clear();

        println!("Planet has spawned!");
    }
}

fn rotate_planets(mut planets_query: Query<(&mut Transform, &Planet)>, time: Res<Time>) {
    for (mut planet_transform, planet_struct) in planets_query.iter_mut() {
        if !planet_struct.is_playing {
            continue;
        }

        planet_transform.rotate_z(-PLANET_ROTATION_SPEED * time.delta_seconds());
    }
}

// TODO: current
fn shrink_current_planet(
    mut commands: Commands,
    mut planets_query: Query<(&mut Sprite, Entity, &mut Collider, &Transform, &mut Planet)>,
    mut planet_spawn_event_writer: EventWriter<PlanetSpawnEvent>,
    mut next_loading_state: ResMut<NextState<LoadingState>>,
    time: Res<Time>,
) {
    for (mut planet_sprite, planet_entity, mut collider, transform, mut planet_struct) in
        planets_query.iter_mut()
    {
        if !planet_struct.is_playing {
            continue;
        }

        let new_planet_size =
            planet_sprite.custom_size.unwrap() - PLANET_SHRINK_SPEED * time.delta_seconds();

        collider.shape.radius -= PLANET_SHRINK_SPEED / 2.0 * time.delta_seconds();

        planet_struct.radius = collider.shape.radius;

        planet_sprite.custom_size = Some(new_planet_size);

        if new_planet_size.distance(PLANET_SHRINK_LIMIT) < 1. {
            // When despawning this entity, other sprites are also despawning for some fucking weird reason.
            for &obstacle_entity in planet_struct.obstacles.iter() {
                commands.entity(obstacle_entity).despawn_recursive();
            }
            commands.entity(planet_entity).despawn_recursive();

            next_loading_state.set(LoadingState::Planet);

            planet_spawn_event_writer.send(PlanetSpawnEvent {
                planet_variant_to_spawn: planet_struct.variant.next(),
                last_planet_position: transform.translation,
            });
        }
    }
}

fn manage_planet_face(
    planet_query: Query<&Planet>,
    mut planet_face_query: Query<(&mut PlanetFace, &mut TextureAtlasSprite)>,
    time: Res<Time>,
) {
    if let Ok(planet_struct) = planet_query.get_single() {
        if let Ok((mut planet_face, mut face_atlas)) = planet_face_query.get_single_mut() {
            if !planet_struct.is_playing {
                return;
            }

            if planet_struct.radius < PLANET_FACE_NORMAL_THRESHOLD {
                face_atlas.index = 1;
                planet_face.face = PlanetFaceState::Normal;
            }
            if planet_struct.radius < PLANET_FACE_BAD_THRESHOLD {
                face_atlas.index = 2;
                planet_face.face = PlanetFaceState::Bad;
            }

            face_atlas.custom_size =
                Some(face_atlas.custom_size.unwrap() - PLANET_SHRINK_SPEED * time.delta_seconds());
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
            transform: Transform::from_translation(PLAYER_START_POSITION),
            ..default()
        },
        Player {
            is_grounded: false,
            velocity: 0.,
        },
        Collider {
            shape: collider_shape,
        },
    ));
}

// TODO: players falls over the planet when pressing S.
fn player_jump(
    mut player_query: Query<(&mut Transform, &mut Player)>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if let Ok((mut player_transform, mut player_struct)) = player_query.get_single_mut() {
        player_struct.velocity += GRAVITY_STRENGTH * GRAVITY_STRENGTH.abs() * time.delta_seconds();

        if keyboard_input.just_pressed(KeyCode::Space) && player_struct.is_grounded {
            player_struct.velocity = PLAYER_JUMP_STRENGTH;
        }

        // accelerate fall
        if keyboard_input.pressed(KeyCode::S) && !player_struct.is_grounded {
            player_struct.velocity += PLAYER_FALL_ACCELERATION * time.delta_seconds();
        }

        player_transform.translation.y += player_struct.velocity * time.delta_seconds();
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
    mut player_query: Query<(&Collider, &mut Transform, &mut Player), Without<Planet>>,
    mut planet_query: Query<(&Collider, &Transform, &mut Planet)>,
) {
    for (player_collider, mut player_transform, mut player_struct) in player_query.iter_mut() {
        for (planet_collider, planet_transform, mut planet_struct) in planet_query.iter_mut() {
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
            let distance = 1.;
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

                player_struct.is_grounded = true;
                planet_struct.is_playing = true;
            } else {
                player_struct.is_grounded = false;
            }

            player_transform.translation = player_translation;
        }
    }
}

fn check_player_obstacle_collisions(
    mut next_app_state: ResMut<NextState<AppState>>,
    mut player_query: Query<(&Collider, &mut Transform), (With<Player>, Without<Obstacle>)>,
    mut obstacle_query: Query<(&Collider, &Transform), With<Obstacle>>,
) {
    for (player_collider, player_transform) in player_query.iter_mut() {
        for (obstacle_collider, obstacle_transform) in obstacle_query.iter_mut() {
            let actor_isometry = Isometry::translation(
                player_transform.translation.x,
                player_transform.translation.y,
            );
            let tile_isometry = Isometry::translation(
                obstacle_transform.translation.x,
                obstacle_transform.translation.y,
            );

            let actor_shape = player_collider.shape.clone_box();
            let tile_shape = obstacle_collider.shape.clone_box();

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
            // TODO: player death
            if let Some(_) = collision {
                next_app_state.set(AppState::GameOver);
                println!("Game Over!");
            }
        }
    }
}

// When the new planet appears, it is filled with new obstacles.
fn spawn_obstacles(
    mut commands: Commands,
    mut planet_query: Query<(&Transform, &mut Planet)>,
    asset_server: Res<AssetServer>,
) {
    println!(
        "Num of planets when spawning obstacles: {}",
        planet_query.iter().len()
    );

    if let Ok((planet_transform, mut planet_struct)) = planet_query.get_single_mut() {
        let mut rng = rand::thread_rng();
        let obstacles_num = rng.gen_range(1..5);

        let mut last_obstacle_angle: f32 = 0.;

        for _ in 0..obstacles_num {
            // Random position on the planet.
            let mut obstacle_position = Vec3::ZERO;
            let mut angle = if rng.gen_bool(0.5) {
                rng.gen_range(0f32..=OBSTACLE_MIN_ANGLE_GENERATION)
            } else {
                rng.gen_range(OBSTACLE_MAX_ANGLE_GENERATION..=2. * PI)
            };

            if last_obstacle_angle != 0. {
                if (angle - last_obstacle_angle).abs() < OBSTACLE_CLOSE_GAP_RANGE.1 {
                    angle -= rng.gen_range(OBSTACLE_CLOSE_GAP_RANGE.0..OBSTACLE_CLOSE_GAP_RANGE.1);
                } else if (angle - last_obstacle_angle).abs() < OBSTACLE_LONG_GAP_RANGE.1 {
                    angle -= rng.gen_range(OBSTACLE_LONG_GAP_RANGE.0..OBSTACLE_LONG_GAP_RANGE.1);
                }
            }

            // angle = angle.clamp(0., OBSTACLE_MAX_ANGLE_GENERATION);

            println!(
                "Last angle | New angle: {} , {}",
                last_obstacle_angle, angle
            );

            last_obstacle_angle = angle;

            let planet_radius = planet_struct.radius;
            let obstacle_radius = OBSTACLE_SIZE.y / 2.;

            obstacle_position.x =
                planet_transform.translation.x + angle.cos() * (planet_radius + obstacle_radius);
            obstacle_position.y =
                planet_transform.translation.y + angle.sin() * (planet_radius + obstacle_radius);

            planet_struct.obstacles.push(
                commands
                    .spawn((
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
                        Obstacle { angle },
                    ))
                    .id(),
            );
        }

        println!("Obstacles have spawned!");
    }
}

fn move_obstacles_on_planet(
    mut children_query: Query<(&mut Transform, &mut Obstacle)>,
    planet_query: Query<(&Planet, &Transform), Without<Obstacle>>,
    time: Res<Time>,
) {
    if let Ok((planet_struct, planet_transform)) = planet_query.get_single() {
        if !planet_struct.is_playing {
            return;
        }

        let planet_translation = planet_transform.translation;
        let planet_radius = planet_struct.radius;

        for &child in planet_struct.obstacles.iter() {
            let child_query = children_query.get_mut(child);

            if let Ok((mut transform, mut obstacle_struct)) = child_query {
                let obstacle_radius = OBSTACLE_SIZE.y / 2.;

                transform.translation.x = planet_translation.x
                    + obstacle_struct.angle.cos() * (planet_radius + obstacle_radius);
                transform.translation.y = planet_translation.y
                    + obstacle_struct.angle.sin() * (planet_radius + obstacle_radius);

                obstacle_struct.angle -= time.delta_seconds() * OBSTACLE_MOVEMENT_SPEED;

                if obstacle_struct.angle.abs() > PI * 2. {
                    obstacle_struct.angle = 0.;
                }
            }
        }
    }
}

// fn manage_game_over() {}
