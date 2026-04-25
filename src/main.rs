use bevy::{input::{common_conditions::input_just_pressed, mouse::AccumulatedMouseMotion}, prelude::*, window::{CursorOptions, PrimaryWindow, WindowFocused}};
use rand::{SeedableRng, seq::IndexedRandom};

fn round_to(value: f32, decimal_places: i32) -> f32 {
    let factor: f32 = 10.0_f32.powi(decimal_places);
    (value * factor).round() / factor
}

#[derive(Debug, Default)]
struct Collisions {
    north: bool,
    south: bool,
    east: bool,
    west: bool,
    up: bool,
    down: bool   
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, (
        spawn_camera,
        spawn_map.after(spawn_camera),
        spawn_menu.after(spawn_camera),
        spawn_hud.after(spawn_camera)));
    app.insert_resource(Time::<Fixed>::from_hz(60.));
    app.add_systems(FixedUpdate,(
        is_collised,
        apply_velocity.after(is_collised),
        apply_gravity.before(apply_velocity).after(is_collised),
        bounce.after(apply_velocity),
        apply_player_velocity.after(is_collised),
        apply_player_gravity.before(apply_player_velocity).after(is_collised)
    ));
    app.add_systems(Update, 
        (player_look,
            player_move.after(player_look),
            focus_event,
            toggle_grab.run_if(input_just_pressed(KeyCode::Escape)),
            spawn_ball,
            shoot_ball.before(spawn_ball).before(focus_event),
            update_power_bar,
            update_player_coords,
            update_menu_visibility,
            update_hud_visibility));
    app.add_observer(apply_grab);
    app.add_message::<BallSpawn>();
    app.init_resource::<BallData>();
    app.insert_resource(Power {
        charging: false,
        current: 0.
    });
    app.run();
}

#[derive(Component)]
struct Player {
    speed: f32,
    creative: bool,
    velocity: Vec3,
}

impl Default for Player {
    fn default() -> Self {
        Player { speed: 50., creative: false, velocity: Vec3::Y * 20. }
    }
}

#[derive(Event, Deref)]
struct GrabEvent(bool);

#[derive(Message)]
struct BallSpawn {
    position: Vec3,
    velocity: Vec3,
    power: f32
}

#[derive(Resource)]
struct BallData {
    mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
    rng: std::sync::Mutex<rand::rngs::StdRng>
}

impl BallData {
    fn mesh(&self) -> Handle<Mesh> {
        self.mesh.clone()
    }
    fn material(&self) -> Handle<StandardMaterial> {
        let mut rng = self.rng.lock().unwrap();
        self.materials.choose(&mut *rng).unwrap().clone()
    }
}

impl FromWorld for BallData {
    fn from_world(world: &mut World) -> Self {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(1.));
        let mut materials = Vec::new();
        let mut mat_assets = world.resource_mut::<Assets<StandardMaterial>>();
        for i in 0..36 {
            let color = Color::hsl((i * 10) as f32, 1., 0.5);
            materials.push(mat_assets.add(StandardMaterial {
                base_color: color,
                ..Default::default()
            }));
        }
        let seed = *b"tunicIsBetterThanYouHEHEHEHAPTDR";
        BallData { mesh, materials, rng: std::sync::Mutex::new(rand::rngs::StdRng::from_seed(seed)) }
    }
}

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec3);

#[derive(Resource)]
struct Power {
    charging: bool,
    current: f32
}

#[derive(Component)]
struct PowerBar {
    min: f32,
    max: f32
}

const NOT_CHARGING: Color = Color::linear_rgb(0.2, 0.2, 0.2);
const MIN_FILL: f32 = 12.5 / 10.;
const EMPTY_SPACE: f32 = 12.5 - MIN_FILL;

#[derive(Component)]
struct Hitbox {
    coords_gap: Vec3,
    size: Vec3,
    collisions: Collisions
}

impl Hitbox {
    fn new(coords_gap: Vec3, x_length: f32, y_length: f32, z_length: f32) -> Self {
        Self {
            coords_gap,
            size: Vec3::new(x_length, y_length, z_length),
            collisions: Collisions::default()
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Transform::from_translation(Vec3::new(0., 5., 0.)),
        Camera3d::default(),
        Player::default(),
        Velocity(Vec3::ZERO),
        Hitbox::new(Vec3::new(0., -2.5, 0.), 2.5, 5., 2.5)
    ));
}

fn spawn_map(
    mut commands: Commands,
    ball_data: Res<BallData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(DirectionalLight::default());
    for h in 0..ball_data.materials.len() {
        commands.spawn((
            Transform::from_translation(Vec3::new((-8. + h as f32) * 2., 5., -30.)),
            Mesh3d(ball_data.mesh()),
            MeshMaterial3d(ball_data.materials[h].clone()),
            Hitbox::new(Vec3::ZERO, 2., 2., 2.)
        ));
    }
    commands.spawn((
        Transform::from_translation(Vec3::new(0., -10., 0.)),
        Mesh3d(meshes.add(Cuboid::new(2500., 20., 2500.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(1., 0., 0.),
            ..Default::default()
        })),
        Hitbox::new(Vec3::ZERO, 500., 20., 500.)
    ));
    commands.spawn((
        Transform::from_translation(Vec3::new(30., 10., 0.)),
        Mesh3d(meshes.add(Cuboid::new(10., 100., 10.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0., 1., 0.),
            ..Default::default()
        })),
        Hitbox::new(Vec3::ZERO, 10., 100., 10.)
    ));
    commands.spawn((
        Transform::from_translation(Vec3::new(-30., 11., 0.)),
        Mesh3d(meshes.add(Cuboid::new(10., 10., 10.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0., 1., 1.),
            ..Default::default()
        })),
        Hitbox::new(Vec3::ZERO, 10., 10., 10.)
    ));
    commands.spawn((
        Transform::from_translation(Vec3::new(-30., 20., -20.)),
        Mesh3d(meshes.add(Cuboid::new(10., 10., 10.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0., 1., 1.),
            ..Default::default()
        })),
        Hitbox::new(Vec3::ZERO, 10., 10., 10.)
    ));
}

#[derive(Component)]
struct MenuUi;

#[derive(Component)]
struct PlayerHud;

#[derive(Component)]
struct CoordsHud;

fn spawn_menu(
    mut commands: Commands,
) {
    commands.spawn((
        MenuUi,
        Node {
        position_type: PositionType::Absolute,
        width: Val::Vw(30.),
        height: Val::Vh(90.),
        bottom: Val::Vh(5.),
        left: Val::Vw(1.5),
        top: Val::Vh(5.),
        flex_direction: FlexDirection::Column,
        border_radius: BorderRadius::all(Val::VMax(1.)),
        ..Default::default()
        },
        BackgroundColor(Color::linear_rgba(0.5, 0.5, 0.5, 0.5)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Controls :"),
            Node {
                margin: UiRect::all(Val::Percent(5.)),
                ..Default::default()
            },
            TextFont {
                font_size: 30.,
                ..Default::default()
            },
            TextColor(Color::linear_rgba(0.75, 0.75, 0.75, 1.))
        ));
        parent.spawn((
            Text::new("- ZQSD/WASD to move\n\
                            - SPACE to jump\n\
                            - LEFT CTRL or Mouse4 to sneak\n\
                            - SHIFT to sprint\n\
                            - LEFT CLICK to throw ball\n\
                            - A to switch between creative/survival mode\n\
                            - ESHAP to show/unshow this menu"),
            Node {
                margin: UiRect::all(Val::Percent(5.)),
                ..Default::default()
            },
            TextFont {
                font_size: 25.,
                ..Default::default()
            },
            TextColor(Color::linear_rgba(0.75, 0.75, 0.75, 1.))
        ));
    });
}

fn spawn_hud(
    mut commands: Commands,
    player: Single<&mut Transform, With<Player>>,
) {
    let pos = player.translation;
    commands.spawn((
        PlayerHud,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Vw(12.5),
            height: Val::Vh(2.5),
            bottom: Val::Vh(5.),
            left: Val::Vw(86.),
            border_radius: BorderRadius::all(Val::VMax(1.)),
            ..Default::default()
        },
        BackgroundColor(Color::linear_rgb(0.5, 0.5, 0.5)),
    )).with_child((
        Node {
            position_type: PositionType::Absolute,
            min_width: Val::Vw(MIN_FILL),
            height: Val::Percent(100.),
            border_radius: BorderRadius::all(Val::VMax(1.)),
            ..Default::default()
        },
        BackgroundColor(NOT_CHARGING),
        PowerBar { min: 1., max: 2.}
    ));
    commands.spawn((
        PlayerHud,
        CoordsHud,
        Text::new(format!("X: {}\nY: {}\nZ: {}", round_to(pos.x, 2), round_to(pos.y, 2), round_to(pos.z, 2))),
        TextFont {
                font_size: 15.,
                ..Default::default()
        },
        TextColor(Color::linear_rgba(0.75, 0.75, 0.75, 1.)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Vh(90.),
            left: Val::Vw(1.),
            ..Default::default()
        }
    ));
}

fn update_menu_visibility(
    cursor: Single<&CursorOptions, (With<PrimaryWindow>, Changed<CursorOptions>)>,
    visibility: Query<&mut Visibility, With<MenuUi>>
) {
    for mut vis in visibility {
        if cursor.visible {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn update_hud_visibility(
    cursor: Single<&CursorOptions, (With<PrimaryWindow>, Changed<CursorOptions>)>,
    visibility: Query<&mut Visibility, With<PlayerHud>>
) {
    for mut vis in visibility {
        if !cursor.visible {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn update_player_coords(
    mut coords: Query<&mut Text, With<CoordsHud>>,
    player: Single<&mut Transform, With<Player>>
) {
    let pos = player.translation;
    for mut text in &mut coords {
        text.0 = format!("X: {}\nY: {}\nZ: {}", round_to(pos.x, 2), round_to(pos.y, 2), round_to(pos.z, 2));
    }
}

fn update_power_bar(
    mut bars: Query<(&mut Node, &PowerBar, &mut BackgroundColor)>,
    power: Res<Power>
) {
    for (mut bar, config, mut bg) in &mut bars {
        if !power.charging {
            bg.0 = NOT_CHARGING;
            bar.width = Val::Vw(MIN_FILL);
        } else {
            let percent = (power.current - config.min) / (config.max - config.min);
            bg.0 = Color::linear_rgb(1. - percent, percent, 0.);
            bar.width = Val::Vw(MIN_FILL + percent * EMPTY_SPACE);
        }
    }
}

fn player_look(
    mut player: Single<&mut Transform, With<Player>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>
) {
    if !window.focused { return;}
    let dt = time.delta_secs();
    let sensitivity = 100. / window.width().min(window.height());
    use EulerRot::YXZ;
    let (mut yaw, mut pitch, _) = player.rotation.to_euler(YXZ);
    pitch -= mouse_motion.delta.y * dt * sensitivity;
    yaw -= mouse_motion.delta.x * dt * sensitivity;
    pitch = pitch.clamp(-1.57, 1.57);
    player.rotation = Quat::from_euler(YXZ, yaw, pitch, 0.);
}

fn apply_grab(
    grab: On<GrabEvent>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>
) {
    use bevy::window::CursorGrabMode;
    if **grab {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    } else {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

fn focus_event(
    mut events: MessageReader<WindowFocused>,
    mut commands: Commands
) {
    if let Some(event) = events.read().last() {
        commands.trigger(GrabEvent(event.focused));
    }
}

fn toggle_grab(
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut commands: Commands
) {
    window.focused = !window.focused;
    commands.trigger(GrabEvent(window.focused));
}

fn player_move(
    player: Single<(&mut Transform, &mut Player, &mut Velocity, &mut Hitbox), With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>
) {
    if cursor.visible {
        return;
    }
    let speed_multiplier = if input.pressed(KeyCode::ShiftLeft) { 3. } else { 1. };
    let mut delta = Vec3::ZERO;
    let (mut transform, mut player_data, mut velocity, hitbox) = player.into_inner();
    if input.pressed(KeyCode::KeyA) {
        delta.x -= 1.;
    }
    if input.pressed(KeyCode::KeyD) {
        delta.x += 1.;
    }
    if input.pressed(KeyCode::KeyW) {
        delta.z += 1.;
    }
    if input.pressed(KeyCode::KeyS) {
        delta.z -= 1.;
    }
    let forward = transform.forward().as_vec3() * delta.z;
    let right = transform.right().as_vec3() * delta.x;
    let mut to_move = forward + right;
    to_move.y = 0.;
    // fly or jump depending on player gamemode
    if player_data.creative && input.pressed(KeyCode::Space) {
        to_move.y += 1.;
    } else if input.pressed(KeyCode::Space) && hitbox.collisions.down {
        velocity.y = player_data.velocity.y;
        to_move.y += 1.;
    }
    if player_data.creative && (input.pressed(KeyCode::ControlLeft) || mouse_input.pressed(MouseButton::Forward)) {
        to_move.y -= 1.;
    }
    if input.just_pressed(KeyCode::KeyQ) {
        player_data.creative = !player_data.creative;
        *velocity = Velocity(Vec3::ZERO);
    }
    to_move = to_move.normalize_or_zero();
    if to_move.x > 0. && hitbox.collisions.east { to_move.x = 0.};
    if to_move.x < 0. && hitbox.collisions.west { to_move.x = 0.};
    if to_move.z > 0. && hitbox.collisions.north { to_move.z = 0.};
    if to_move.z < 0. && hitbox.collisions.south { to_move.z = 0.};
    transform.translation += to_move * time.delta_secs() * player_data.speed * speed_multiplier;
    if !player_data.creative && (input.just_pressed(KeyCode::ControlLeft) || mouse_input.just_pressed(MouseButton::Forward)) {
        transform.translation.y -= 1.;
        player_data.speed *= 0.25;
    } else if !player_data.creative && (input.just_released(KeyCode::ControlLeft) || mouse_input.just_released(MouseButton::Forward)) {
        transform.translation.y += 1.;
        player_data.speed *= 4.;
    }
}

fn spawn_ball(
    mut events: MessageReader<BallSpawn>,
    mut commands: Commands,
    ball_data: Res<BallData>
) {
    for spawn in events.read() {
        commands.spawn((
            Transform::from_translation(spawn.position),
            Mesh3d(ball_data.mesh()),
            MeshMaterial3d(ball_data.material()),
            Velocity(spawn.velocity * spawn.power * 5.),
            Hitbox::new(Vec3::ZERO, 2., 2., 2.)
        ));
    }

}

fn shoot_ball(
    mouse_inputs: Res<ButtonInput<MouseButton>>,
    player: Single<(&mut Transform, &mut Player), With<Player>>,
    mut spawner: MessageWriter<BallSpawn>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    mut power: ResMut<Power>,
    time: Res<Time>
) {
    if cursor.visible {
        return;
    }
    if power.charging {
        if mouse_inputs.just_released(MouseButton::Left) {
            spawner.write(BallSpawn {
                position: player.0.translation,
                velocity: player.0.forward().as_vec3() * 2.5,
                power: (power.current * 2.).exp()
            });
        }
        if mouse_inputs.pressed(MouseButton::Left) {
            power.current += time.delta_secs();
            power.current = power.current.clamp(1., 2.);
        } else {
            power.charging = false;
        }
    }
    if mouse_inputs.just_pressed(MouseButton::Left) {
        power.charging = true;
        power.current = 1.;
    }
}

fn apply_velocity(
    mut objects: Query<(&mut Transform, &Velocity), Without<Player>>,
    time: Res<Time>
) {
    for (mut transform, velocity) in &mut objects {
        transform.translation += velocity.0 * time.delta_secs();
    }
}

const GRAVITY: Vec3 = Vec3::new(0., -9.8, 0.);
fn apply_gravity(
    mut objects: Query<(&mut Velocity, &Hitbox), Without<Player>>,
    time: Res<Time>
) {
    let g = GRAVITY * time.delta_secs() * 50.;
    for (mut v, hitbox) in &mut objects {
        if !hitbox.collisions.down {
            **v += g;
        }
    }
}

fn bounce(
    mut balls: Query<(&Hitbox, &mut Velocity), Without<Player>>,
) {
    for (hitbox, mut velocity) in &mut balls {
        if hitbox.collisions.down || hitbox.collisions.up {
            velocity.y *= -0.75;
        }
        velocity.x *= 0.99;
        velocity.z *= 0.99;
    }
}

fn apply_player_velocity(
    mut players: Query<(&mut Transform, &Velocity, &Player)>,
    time: Res<Time>
) {
    for (mut transform, velocity, player_data) in &mut players {
        if !player_data.creative {
            transform.translation += velocity.0 * time.delta_secs();
        }
    }
}

fn apply_player_gravity(
    mut players: Query<(&mut Velocity, &Hitbox, &Player)>,
    time: Res<Time>
) {
    let g = GRAVITY * time.delta_secs() * 10.;
    for (mut velocity, hitbox, player) in &mut players {
        if !player.creative {
            if hitbox.collisions.down {
                **velocity *= 0.;
            } else if hitbox.collisions.up {
                **velocity *= 0.;
                **velocity += g;
            } else {
                **velocity += g;
            }
        }
    }
}

fn is_collised(
    objects: Query<(&Transform, &Hitbox), Without<Velocity>>, 
    mut moving_objects: Query<(&Transform, &mut Hitbox, &Velocity)>,
    time: Res<Time>
) {
    for (mov_coords, mut mov_hit, velo) in &mut moving_objects {
        let center1 = mov_coords.translation + mov_hit.coords_gap + **velo * time.delta_secs();
        let a_min = center1 - mov_hit.size / 2.;
        let a_max = center1 + mov_hit.size / 2.;
        let mut collides = Collisions::default();

        for (object_coords, object_hit) in &objects {
            let center2 = object_coords.translation + object_hit.coords_gap;
            let b_min = center2 - object_hit.size / 2.;
            let b_max = center2 + object_hit.size / 2.;

            // Chevauchement sur chaque axe
            let overlap_x = a_max.x.min(b_max.x) - a_min.x.max(b_min.x);
            let overlap_y = a_max.y.min(b_max.y) - a_min.y.max(b_min.y);
            let overlap_z = a_max.z.min(b_max.z) - a_min.z.max(b_min.z);

            // Collision seulement si les 3 axes se chevauchent
            if overlap_x > 0. && overlap_y > 0. && overlap_z > 0. {
                // L'axe avec le plus petit overlap = direction de collision
                if overlap_y <= overlap_x && overlap_y <= overlap_z {
                    // Collision verticale
                    if center1.y > center2.y {
                        collides.down = true; // sol
                    } else {
                        collides.up = true;   // plafond
                    }
                } else if overlap_x <= overlap_y && overlap_x <= overlap_z {
                    // Collision horizontale X
                    if center1.x > center2.x {
                        println!("Tu touches l'ouest mec");
                        collides.west = true;
                    } else {
                        println!("Tu touches l'est mec");
                        collides.east = true;
                    }
                } else {
                    // Collision horizontale Z
                    if center1.z > center2.z {
                        println!("Tu touches le sud mec");
                        collides.south = true;
                    } else {
                        println!("Tu touches le nord mec");
                        collides.north = true;
                    }
                }
            }
        }
        mov_hit.collisions = collides;
    }
}