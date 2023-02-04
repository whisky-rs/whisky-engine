use crossbeam::channel::{self, Sender};
use winit::{event::{ElementState, MouseButton, KeyboardInput}, dpi::{PhysicalPosition, PhysicalSize}};

use crate::{geometry::{Circle, Point}, InputMessage};
use std::time::{Instant, Duration};

pub struct GameState {
    pub mouse_position: [f32; 2],
    pub timer: Instant,
    pub player: Circle,
    pub angle: f32,
    pub reset_position: bool,
}

impl GameState {
    pub fn handle_mouse_moved(&mut self, position: PhysicalPosition<f64>, dimensions: PhysicalSize<u32>, input_physics_actions: &mut channel::Sender<InputMessage>) {
        if self.timer.elapsed() <= Duration::from_millis(100) {
            // have to normalize coordinates
            self.mouse_position = Self::normalize_mouse_position(dimensions, position);

            self.calculate_new_angle();
            input_physics_actions.send(InputMessage::Angle(self.angle)).unwrap();

            self.reset_position = true;
            self.timer = Instant::now();
        }
    }

    pub fn handle_keyboard_input(&mut self, input: KeyboardInput, input_physics_actions: &mut channel::Sender<InputMessage>) {
        match input {
            KeyboardInput {
                state: ElementState::Pressed,
                virtual_keycode: Some(winit::event::VirtualKeyCode::Space),
                ..
            } => {
                input_physics_actions.send(InputMessage::Jump).unwrap();
            },
            _ => {}
        };
    }

    fn calculate_new_angle(&mut self) {
        let two_pi = 2. * std::f32::consts::PI;
        self.angle = (self.angle + self.mouse_position[0] * std::f32::consts::PI) % two_pi;

    }

    fn normalize_mouse_position(
        dimensions: PhysicalSize<u32>,
        mouse_position: PhysicalPosition<f64>,
    ) -> [f32; 2] {
        [
            (mouse_position.x * 2.0 - dimensions.width as f64) as f32 / dimensions.width as f32,
            (mouse_position.y * 2.0 - dimensions.height as f64) as f32 / dimensions.height as f32,
        ]
    }

}

