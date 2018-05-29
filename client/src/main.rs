extern crate ggez;
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
    nalgebra::{Point3, Vector3, Vector2},
    slog::{Logger},
    noise::{NoiseFn, HybridMulti},

    lagato::{camera::{PitchYawCamera}, grid::{Voxels, Range}, DirectionalInput, rotate_vector},
    blockengine::{Chunk},
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
            let direction = self.camera.to_quaternion() * Vector3::new(0.0, 0.0, -1.0);
            for chunk in &self.chunks {
                // Offset the position for the ray trace
                let chunk_position = Vector3::new(
                    chunk.position.x as f32, chunk.position.y as f32, chunk.position.z as f32
                ) * 16.0;
                let origin = camera_position - chunk_position;

                // Cast the ray
                let result = cast_ray(origin, direction, 10.0, &chunk.voxels);
                if let Some((position, _normal)) = result {
                    self.objects[self.pointer_object].position =
                        Point3::new(
                            // A little bit added so we can see it
                            position.x as f32 + 0.2,
                            position.y as f32 + 0.2,
                            position.z as f32 + 0.2,
                        ) +
                        chunk_position;
                    break
                }

                // TODO: Make sure we find the closest ray hit, right now just the first chunk hit
                // is used
            }

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

fn cast_ray(
    origin: Point3<f32>, direction: Vector3<f32>, mut radius: f32, voxels: &Voxels<bool>,
) -> Option<(Point3<i32>, Vector3<f32>)> {
    // Cube containing origin point
    let mut voxel = Point3::new(
        origin.x.floor() as i32, origin.y.floor() as i32, origin.z.floor() as i32
    );

    // Direction to increment x,y,z when stepping
    let step = Vector3::new(signum(direction.x), signum(direction.y), signum(direction.z));

    // T when reaching the next voxel on an axis
    let mut t_max = Vector3::new(
        intbound(origin.x, direction.x),
        intbound(origin.y, direction.y),
        intbound(origin.z, direction.z),
    );

    // The change in t when taking a step (always positive)
    let t_delta = Vector3::new(
        step.x as f32 / direction.x,
        step.y as f32 / direction.y,
        step.z as f32 / direction.z,
    );

    let mut normal = Vector3::new(0.0, 0.0, 0.0);

    // Avoids an infinite loop.
    if direction.x == 0.0 && direction.y == 0.0 && direction.z == 0.0 {
        panic!("Raycast in zero direction!")
    }

    // Rescale from units of 1 cube-edge to units of 'direction' so we can
    // compare with 't'
    radius /= (direction.x*direction.x + direction.y*direction.y + direction.z*direction.z).sqrt();

    while is_in_bounds_step(step, voxels.size(), voxel) {
        // If it's solid, we're done
        if let Ok(true) = voxels.get(voxel) {
            return Some((voxel, normal))
        }

        // t_max.x stores the t-value at which we cross a cube boundary along the
        // X axis, and similarly for Y and Z. Therefore, choosing the least t_max
        // chooses the closest cube boundary. Only the first case of the four
        // has been commented in detail.
        if t_max.x < t_max.y {
            if t_max.x < t_max.z {
                if t_max.x > radius { break }
                // Update which cube we are now in.
                voxel.x += step.x;
                // Adjust t_max.x to the next X-oriented boundary crossing.
                t_max.x += t_delta.x;
                // Record the normal vector of the cube face we entered.
                normal = Vector3::new(-step.x as f32, 0.0, 0.0);
            } else {
                if t_max.z > radius { break }
                voxel.z += step.z;
                t_max.z += t_delta.z;
                normal = Vector3::new(0.0, 0.0, -step.z as f32);
            }
        } else {
            if t_max.y < t_max.z {
                if t_max.y > radius { break }
                voxel.y += step.y;
                t_max.y += t_delta.y;
                normal = Vector3::new(0.0, -step.y as f32, 0.0);
            } else {
                // Identical to the second case, repeated for simplicity in
                // the conditionals.
                if t_max.z > radius { break }
                voxel.z += step.z;
                t_max.z += t_delta.z;
                normal = Vector3::new(0.0, 0.0, -step.z as f32);
            }
        }
    }

    None
}

fn is_in_bounds_step(step: Vector3<i32>, size: Vector3<i32>, voxel: Point3<i32>) -> bool {
    let x = if step.x > 0 { voxel.x < size.x } else { voxel.x >= 0 };
    let y = if step.y > 0 { voxel.y < size.y } else { voxel.y >= 0 };
    let z = if step.z > 0 { voxel.z < size.z } else { voxel.z >= 0 };
    x && y && z
}

fn signum(x: f32) -> i32 {
    if x > 0.0 {
        1
    } else {
        if x < 0.0 {
            -1
        } else {
            0
        }
    }
}

fn intbound(mut s: f32, ds: f32) -> f32 {
    // Find the smallest positive t such that s+t*ds is an integer
    if ds < 0.0 {
        intbound(-s, -ds)
    } else {
        s = modulus(s, 1.0);
        // problem is now s+t*ds = 1
        (1.0 - s) / ds
    }
}

fn modulus(value: f32, modulus: f32) -> f32 {
    // This is different but I'm not sure in what way
    (value % modulus + modulus) % modulus
}
