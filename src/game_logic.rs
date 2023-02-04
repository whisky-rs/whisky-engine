use crossbeam::channel;
use winit::{event::{ElementState, MouseButton, KeyboardInput}, dpi::{PhysicalPosition, PhysicalSize}};

use crate::{geometry::{Circle, Point}, InputMessage};
use std::time::{Instant, Duration};

#[derive(Clone, Copy)]
pub enum Tool {
    Crayon,
    Rigid,
    Hinge,
    Eraser,
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub is_deadly: bool,
    pub is_fragile: bool,
    pub free_quad: Vec<[f32; 2]>
}

pub struct GameStateProperties {
    pub mouse_position: [f32; 2],
    pub mpsaved: [f32; 2],
    pub line_points: Vec<[f32; 2]>,
    pub static_circle: Circle,
    pub is_beginning_draw: bool,
    pub is_mouse_clicked: bool,
    pub is_holding: bool,
    pub ed: EditorState,
    pub timer: Instant,
    pub tool: Tool,
}

pub struct GameState(pub GameStateProperties,);

impl GameState {
    pub fn handle_mouse_input(&mut self, element_state: ElementState, button: MouseButton, input_physics_actions: &mut channel::Sender<InputMessage>) {
        if button == MouseButton::Left && element_state == ElementState::Pressed {
            let [x, y] = self.0.mouse_position;
            let mouse = Point(x as f64, -y as f64);
            match self.0.tool {
                Tool::Eraser => {
                    input_physics_actions.send(InputMessage::Erase(mouse)).unwrap();
                }
                Tool::Hinge => {
                    input_physics_actions.send(InputMessage::Hinge(mouse)).unwrap();
                }
                Tool::Rigid => {
                    input_physics_actions.send(InputMessage::Rigid(mouse)).unwrap();
                }
                _ => {}
            };

            self.0.is_mouse_clicked = true;
            if !self.0.is_holding {
                self.0.static_circle.center = Point(
                    self.0.mouse_position[0] as f64,
                    -self.0.mouse_position[1] as f64,
                );
            };
            self.0.is_holding = true;

            self.0.timer = Instant::now();
        }
        if button == MouseButton::Left && element_state == ElementState::Released {
            if let Tool::Crayon = self.0.tool {
                if self.0.is_holding {
                    input_physics_actions
                        .send(InputMessage::DrawCircle(self.0.static_circle))
                        .unwrap();
                    self.0.static_circle.radius = 0.;
                } else {
                    if self.0.line_points.len() > 20 {
                        input_physics_actions
                            .send(InputMessage::DrawPolygon(std::mem::take(
                                &mut self.0.line_points,
                            )))
                            .unwrap();
                    } else {
                        self.0.line_points.clear();
                    }

                    self.0.line_points.push([0.0, 0.0]);
                    self.0.line_points.push([0.0, 0.0]);
                }
            }

            self.0.is_mouse_clicked = false;
            self.0.is_beginning_draw = true;
            self.0.is_holding = false;
        }
        if button == MouseButton::Right && element_state == ElementState::Pressed {
            self.0.mpsaved = self.0.mouse_position;
            eprintln!("aa");
        }
        if button == MouseButton::Middle && element_state == ElementState::Pressed {
            let [mut x1,mut y1] = self.0.mouse_position;
            let [mut x2,mut y2] = self.0.mpsaved;

            if x1.abs() > 0.95 {
                x1 *= 1.5
            }
            if y1.abs() > 0.95 {
                y1 *= 1.5
            }
            if x2.abs() > 0.95 {
                x2 *= 1.5
            }
            if y2.abs() > 0.95 {
                y2 *= 1.5
            }
            
            input_physics_actions.send(InputMessage::CreateLevelShape([x1,-y1], [x2,-y2], self.0.ed.clone())).unwrap();
        }
    }

    pub fn handle_mouse_moved(&mut self, position: PhysicalPosition<f64>, dimensions: PhysicalSize<u32>) {
         // have to normalize coordinates
         self.0.mouse_position = Self::normalize_mouse_position(dimensions, position);
         if let Tool::Crayon = self.0.tool {
             if self.0.timer.elapsed() <= Duration::from_millis(500) {
                 self.0.is_holding = false;
                 self.0.static_circle.radius = 0.;
             }

             if self.0.is_holding {
                 return;
             }
             if self.0.is_beginning_draw && self.0.is_mouse_clicked {
                 self.0.line_points.clear();
                 self.0.line_points.push(self.0.mouse_position);
                 self.0.is_beginning_draw = false;
             }

             if self.0.is_mouse_clicked {
                 self.0.line_points.push(self.0.mouse_position);
             }
         }
    }

    pub fn handle_keyboard_input(&mut self, input: KeyboardInput, input_physics_actions: &mut channel::Sender<InputMessage>) {
        self.0.tool = match input {
            KeyboardInput {
                state: ElementState::Pressed,
                virtual_keycode: Some(winit::event::VirtualKeyCode::A),
                ..
            } => Tool::Eraser,
            KeyboardInput {
                state: ElementState::Pressed,
                virtual_keycode: Some(winit::event::VirtualKeyCode::D),
                ..
            } => Tool::Hinge,
            KeyboardInput {
                state: ElementState::Pressed,
                virtual_keycode: Some(winit::event::VirtualKeyCode::S),
                ..
            } => Tool::Rigid,
            KeyboardInput {
                state: ElementState::Released,
                virtual_keycode:
                    Some(
                        winit::event::VirtualKeyCode::A
                        | winit::event::VirtualKeyCode::S
                        | winit::event::VirtualKeyCode::D,
                    ),
                ..
            } => Tool::Crayon,
            KeyboardInput {
                state: ElementState::Released,
                virtual_keycode:
                    Some(
                        winit::event::VirtualKeyCode::P
                    ),
                ..
            } => {input_physics_actions.send(InputMessage::RemoveLastShape).unwrap(); self.0.tool}
            KeyboardInput {
                state: ElementState::Released,
                virtual_keycode:
                    Some(
                        winit::event::VirtualKeyCode::O
                    ),
                ..
            } => {self.0.ed.is_deadly = !self.0.ed.is_deadly; self.print_editor_state(); self.0.tool}
            KeyboardInput {
                state: ElementState::Released,
                virtual_keycode:
                    Some(
                        winit::event::VirtualKeyCode::L
                    ),
                ..
            } => {self.0.ed.is_fragile = !self.0.ed.is_fragile; self.print_editor_state(); self.0.tool}
            KeyboardInput {
                state: ElementState::Released,
                virtual_keycode:
                    Some(
                        winit::event::VirtualKeyCode::N
                    ),
                ..
            } => {
                self.0.ed.free_quad.push(self.0.mouse_position);
                if self.0.ed.free_quad.len() == 4 {
                    input_physics_actions.send(InputMessage::CreateLevelShapeFreeQuad(self.0.ed.clone())).unwrap();
                    self.0.ed.free_quad.clear();
                }
                self.0.tool
            }
            _ => self.0.tool,
        };
    }

    fn print_editor_state(&self) {
        eprintln!("{:?}", self.0.ed)
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

