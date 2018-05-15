extern crate ggez;
extern crate nalgebra;
#[macro_use] extern crate slog;
extern crate noise;
extern crate lagato;
extern crate lagato_ggez;
extern crate blockengine;

use {
    ggez::{
        event::{EventHandler, MouseButton, MouseState},
        timer, mouse,
        Context, GameResult,
    },
    nalgebra::{Point3, Vector3, UnitQuaternion},
    slog::{Logger},
    noise::{NoiseFn, HybridMulti},

    lagato::{camera::{PitchYawCamera}, grid::{Voxels}},
    blockengine::{rendering::{Renderer, RenderCamera}},
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

    world: Voxels<bool>,
    camera: PitchYawCamera,
    player_position: Point3<f32>,
}

impl MainState {
    fn new(ctx: &mut Context, log: Logger) -> GameResult<MainState> {
        info!(log, "Loading game");

        mouse::set_relative_mode(ctx, true);

        // Create and generate world
        let size = Vector3::new(128, 32, 128);
        let mut world = Voxels::empty(size);
        let noise = HybridMulti::new();
        for x in 0..size.x {
            for z in 0..size.z {
                let value = noise.get([x as f64 * 0.005, z as f64 * 0.005]);

                // Re-range the value to between 0 and 1
                let ranged_value = (value + 1.0) / 2.0;
                let clamped_value = ranged_value.min(1.0).max(0.0);

                let height = ((size.y-1) as f64 * clamped_value).round() + 1.0;

                for y in 0..height as i32 {
                    *world.get_mut(Point3::new(x, y, z)).unwrap() = true;
                }
            }
        }

        let renderer = Renderer::new(ctx, &world);

        let player_position = Point3::new(0.0, 40.0, 0.0);
        let camera = PitchYawCamera::new(0.0, 0.0);

        Ok(MainState {
            log,
            renderer,

            world,
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
            //self.camera.yaw += DELTA;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let render_camera = RenderCamera::new(
            self.player_position + Vector3::new(0.0, 1.5, 0.0),
            UnitQuaternion::from_euler_angles(self.camera.pitch, self.camera.yaw, 0.0),
        );

        self.renderer.draw(ctx, &render_camera)?;

        Ok(())
    }

    fn mouse_button_down_event(
        &mut self, _ctx: &mut Context,
        _button: MouseButton, _x: i32, _y: i32
    ) {
    }

    fn mouse_button_up_event(
        &mut self, _ctx: &mut Context,
        _button: MouseButton, _x: i32, _y: i32
    ) {
    }

    fn mouse_motion_event(
        &mut self, _ctx: &mut Context,
        _state: MouseState, _x: i32, _y: i32, xrel: i32, yrel: i32
    ) {
        let sensitivity = 0.0025;

        self.camera.yaw += xrel as f32 * -sensitivity;
        self.camera.pitch += yrel as f32 * -sensitivity;

        let limit = ::std::f32::consts::PI * 0.475;
        self.camera.pitch = self.camera.pitch.max(-limit).min(limit);
    }

    fn quit_event(&mut self, _ctx: &mut Context) -> bool {
        info!(self.log, "quit_event() callback called, quitting");
        false
    }
}
