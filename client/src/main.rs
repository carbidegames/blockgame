extern crate ggez;
extern crate nalgebra;
#[macro_use] extern crate slog;
extern crate noise;
extern crate udpcon;
extern crate lagato;
extern crate lagato_ggez;
extern crate blockengine;
extern crate blockgame_server;

use {
    ggez::{
        event::{EventHandler, Keycode, Mod, MouseState},
        timer, mouse,
        Context, GameResult,
    },
    nalgebra::{Point3, Vector3, Vector2},
    slog::{Logger},
    noise::{NoiseFn, HybridMulti},

    udpcon::{Peer, Event},
    lagato::{camera::{PitchYawCamera}, grid::{Voxels}},
    blockengine::{rendering::{Renderer, VoxelsMesh}, Chunk},
};

pub fn main() -> GameResult<()> {
    lagato_ggez::run_game(
        "blockgame", "carbidegames", "Block Game",
        |ctx, log| MainState::new(ctx, log),
    )
}

struct MainState {
    log: Logger,
    renderer: Renderer,
    input: InputState,
    client: Peer,

    chunks: Vec<Chunk>,
    camera: PitchYawCamera,
    player_position: Point3<f32>,
}

impl MainState {
    fn new(ctx: &mut Context, log: Logger) -> GameResult<MainState> {
        info!(log, "Loading game");

        mouse::set_relative_mode(ctx, true);

        let renderer = Renderer::new(ctx);

        // Create and generate world
        let chunk_size = Vector3::new(16, 32, 16);
        let noise_multiply = 0.005;
        let noise = HybridMulti::new();

        let mut chunks = Vec::new();
        // TODO: Restructure bounds to any kind of cell range
        for chunk_x in -3..4 {
            for chunk_z in -3..4 {
                let mut chunk_voxels = Voxels::empty(chunk_size);
                for x in 0..chunk_size.x {
                    for z in 0..chunk_size.z {
                        let total_x = (chunk_x * chunk_size.x + x) as f64;
                        let total_z = (chunk_z * chunk_size.z + z) as f64;
                        let value = noise.get([
                            total_x * noise_multiply,
                            total_z * noise_multiply,
                        ]);

                        // Re-range the value to between 0 and 1
                        let ranged_value = (value + 1.0) / 2.0;
                        let clamped_value = ranged_value.min(1.0).max(0.0);

                        let height = ((chunk_size.y-1) as f64 * clamped_value).round() + 1.0;

                        for y in 0..height as i32 {
                            *chunk_voxels.get_mut(Point3::new(x, y, z)).unwrap() = true;
                        }
                    }
                }

                let mesh = VoxelsMesh::triangulate(ctx, &chunk_voxels);
                chunks.push(Chunk {
                    position: Vector2::new(chunk_x, chunk_z),
                    voxels: chunk_voxels,
                    mesh,
                });
            }
        }

        let player_position = Point3::new(0.0, 40.0, 0.0);
        let camera = PitchYawCamera::new(0.0, 0.0);

        let server = "127.0.0.1:25566".parse().unwrap();
        let mut client = Peer::start(None, blockgame_server::PROTOCOL);
        client.send(server, [0, 1, 2, 3].to_vec()).unwrap();
        client.send(server, [3, 0, 1, 2].to_vec()).unwrap();

        Ok(MainState {
            log,
            renderer,
            input: InputState::new(),
            client,

            chunks,
            player_position,
            camera,
        })
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        const DESIRED_FPS: u32 = 60;
        const DELTA: f32 = 1.0 / DESIRED_FPS as f32;

        while timer::check_update_time(ctx, DESIRED_FPS) {
            for event in self.client.poll() {
                match event {
                    Event::Message { source, data } =>
                        info!(self.log, "Data: {:?} from {}", data, source),
                    Event::NewPeer { address } =>
                        info!(self.log, "Server Connected: {}", address),
                    Event::PeerTimedOut { address } =>
                        info!(self.log, "Server Disconnected: {}", address),
                }
            }

            let mut input = Vector2::new(0.0, 0.0);
            if self.input.backward { input.y += 1.0; }
            if self.input.forward { input.y -= 1.0; }
            if self.input.left { input.x -= 1.0; }
            if self.input.right { input.x += 1.0; }
            if input.x != 0.0 || input.y != 0.0 {
                input = input.normalize();
            }

            rotate(&mut input, -self.camera.yaw);

            const SPEED: f32 = 2.0;
            self.player_position.x += input.x * DELTA * SPEED;
            self.player_position.z += input.y * DELTA * SPEED;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let render_camera = self.camera.to_render_camera(
            self.player_position + Vector3::new(0.0, 1.5, 0.0)
        );

        self.renderer.draw(ctx, &render_camera, &self.chunks)?;

        Ok(())
    }

    fn key_down_event(
        &mut self, ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool
    ) {
        match keycode {
            Keycode::S => self.input.backward = true,
            Keycode::W => self.input.forward = true,
            Keycode::A => self.input.left = true,
            Keycode::D => self.input.right = true,
            Keycode::Escape => ctx.quit().unwrap(),
            _ => {}
        }
    }

    fn key_up_event(
        &mut self, _ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool
    ) {
        match keycode {
            Keycode::S => self.input.backward = false,
            Keycode::W => self.input.forward = false,
            Keycode::A => self.input.left = false,
            Keycode::D => self.input.right = false,
            _ => {}
        }
    }

    fn mouse_motion_event(
        &mut self, _ctx: &mut Context,
        _state: MouseState, _x: i32, _y: i32, xrel: i32, yrel: i32
    ) {
        self.camera.handle_mouse_motion(Vector2::new(xrel, yrel));
    }

    fn quit_event(&mut self, _ctx: &mut Context) -> bool {
        info!(self.log, "quit_event() callback called, quitting");
        false
    }
}

struct InputState {
    backward: bool,
    forward: bool,
    left: bool,
    right: bool,
}

impl InputState {
    pub fn new() -> Self {
        InputState {
            backward: false,
            forward: false,
            left: false,
            right: false,
        }
    }
}

fn rotate(value: &mut Vector2<f32>, radians: f32) {
    let sin = radians.sin();
    let cos = radians.cos();

    let tx = value.x;
    let ty = value.y;

    value.x = (cos * tx) - (sin * ty);
    value.y = (sin * tx) + (cos * ty);
}
