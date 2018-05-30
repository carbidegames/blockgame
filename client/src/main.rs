extern crate ggez;
extern crate alga;
extern crate nalgebra;
#[macro_use] extern crate slog;
extern crate noise;
extern crate udpcon;
extern crate lagato;
extern crate lagato_ggez;
extern crate blockengine;
extern crate blockengine_rendering;
extern crate blockgame_server;

mod networking;

use {
    ggez::{
        event::{EventHandler, Keycode, Mod, MouseState},
        timer, mouse,
        Context, GameResult,
    },
    alga::linear::{EuclideanSpace},
    nalgebra::{Point3, Vector3, Vector2},
    slog::{Logger},
    noise::{NoiseFn, HybridMulti},

    lagato::{camera::{PitchYawCamera}, grid::{Voxels, Range}, DirectionalInput, rotate_vector},
    blockengine::{cast_ray},
    blockengine_rendering::{Renderer, Texture, Mesh, Object, triangulate_voxels},

    networking::{Connection},
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
    input: DirectionalInput,
    connection: Connection,

    chunks: Vec<Chunk>,
    objects: Vec<Object>,
    pointer_object: usize,
    camera: PitchYawCamera,
    player_position: Point3<f32>,
}

impl MainState {
    fn new(ctx: &mut Context, log: Logger) -> GameResult<MainState> {
        info!(log, "Loading game");

        mouse::set_relative_mode(ctx, true);

        let block_texture = Texture::load(ctx, "/dirt.png")?;
        let renderer = Renderer::new(ctx, &block_texture);
        let input = DirectionalInput::new();

        // Create and generate world
        let chunk_size = Vector3::new(16, 16, 16);
        let height_in_chunks = 4;
        let noise_multiply = 0.005;
        let noise = HybridMulti::new();

        let mut chunks = Vec::new();
        let mut objects = Vec::new();
        for chunk_column in Range::new_dim2(-4, -4, 3, 3).iter() {
            let mut column = Vec::new();

            for _ in 0..height_in_chunks {
                column.push(Voxels::empty(chunk_size));
            }

            // Go through all top-down grid positions in the column
            for local_position in Range::new_dim2(0, 0, chunk_size.x-1, chunk_size.z-1).iter() {
                let total_x = (chunk_column.x * chunk_size.x + local_position.x) as f64;
                let total_z = (chunk_column.y * chunk_size.z + local_position.y) as f64;
                let value = noise.get([
                    total_x * noise_multiply,
                    total_z * noise_multiply,
                ]);

                // Re-range the value to between 0 and 1
                let ranged_value = (value + 1.0) / 2.0;
                let clamped_value = ranged_value.min(1.0).max(0.0);

                let max_height = height_in_chunks * chunk_size.y;
                let height = ((max_height-1) as f64 * clamped_value).round() + 1.0;
                let height = height as i32;

                // Go through all blocks at this x y point
                for y in 0..height {
                    let chunk_y = y / chunk_size.y;
                    let local_y = y % chunk_size.y;
                    let voxels = &mut column[chunk_y as usize];

                    let voxel_position = Point3::new(local_position.x, local_y, local_position.y);
                    *voxels.get_mut(voxel_position).unwrap() = true;
                }
            }

            for (i, voxels) in column.into_iter().enumerate() {
                let mesh = Mesh::new(ctx, &triangulate_voxels(&voxels));
                chunks.push(Chunk {
                    position: Point3::new(chunk_column.x, i as i32, chunk_column.y),
                    voxels,
                });
                objects.push(Object {
                    position: Point3::new(
                        (chunk_column.x * 16) as f32, (i * 16) as f32, (chunk_column.y * 16) as f32
                    ),
                    mesh,
                });
            }
        }

        // Create the object we'll use to show where the player is pointing
        objects.push(Object {
            position: Point3::new(0.0, 0.0, 0.0),
            mesh: Mesh::cube(ctx),
        });
        let pointer_object = objects.len() - 1;

        let player_position = Point3::new(0.0, 40.0, 0.0);
        let camera = PitchYawCamera::new(0.0, 0.0);

        let connection = Connection::new();

        Ok(MainState {
            log,
            renderer,
            input,
            connection,

            chunks,
            objects,
            pointer_object,
            player_position,
            camera,
        })
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        const DESIRED_FPS: u32 = 60;
        const _DELTA: f32 = 1.0 / DESIRED_FPS as f32;

        while timer::check_update_time(ctx, DESIRED_FPS) {
            self.connection.update(&self.log, &mut self.player_position);

            // Check where in the world we're aiming at
            let camera_position = self.player_position + Vector3::new(0.0, 1.5, 0.0);
            let direction = self.camera.to_rotation() * Vector3::new(0.0, 0.0, -1.0);
            let mut found = Point3::new(0.0, 0.0, 0.0);
            let mut found_distance_sqr = 100.0*100.0;
            for chunk in &self.chunks {
                // Offset the position for the ray trace
                let chunk_position = Vector3::new(
                    chunk.position.x as f32, chunk.position.y as f32, chunk.position.z as f32
                ) * 16.0;
                let origin = camera_position - chunk_position;

                // Cast the ray
                let result = cast_ray(origin, direction, 10.0, &chunk.voxels);
                if let Some((position, _normal)) = result {
                    let new_found = Point3::new(
                        position.x as f32,
                        position.y as f32,
                        position.z as f32,
                    ) + chunk_position;
                    let distance_sqr = new_found.distance_squared(&camera_position);
                    if distance_sqr < found_distance_sqr {
                        found = new_found;
                        found_distance_sqr = distance_sqr;
                    }
                }
            }

            // A little bit added so we can see it
            let offset = Vector3::new(0.2, 0.2, 0.2);
            self.objects[self.pointer_object].position = found + offset;

            // Calculate which direction we need to move based on the current input
            let mut input = self.input.to_vector();
            input = rotate_vector(input, -self.camera.yaw);

            // Send that over to the server
            self.connection.send_input(input);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let render_camera = self.camera.to_render_camera(
            self.player_position + Vector3::new(0.0, 1.5, 0.0)
        );

        self.renderer.draw(ctx, &render_camera, &self.objects)?;

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

        self.connection.stop();

        false
    }
}

pub struct Chunk {
    pub position: Point3<i32>,
    pub voxels: Voxels<bool>,
}
