use crossbeam::channel::{self, TryRecvError};
use game_logic::{GameState, GameStateProperties, Tool};
use geometry::Point;
use levels::{Level, LoadError};
use std::{env, thread, time::{Duration, Instant}};

use physics::{compute, shape::Circle};

pub mod geometry;
pub mod graphics_engine;
pub mod levels;
pub mod physics;
pub mod game_logic;

pub enum InputMessage {
    Erase(Point),
    Rigid(Point),
    Hinge(Point),
    DrawPolygon(Vec<[f32; 2]>),
    DrawCircle(geometry::Circle),
}

#[derive(Debug, thiserror::Error)]
pub enum ArgError {
    #[error("missing first argument - path to level file")]
    MissingFileName,
    #[error(transparent)]
    Load(#[from] LoadError),
}

#[doc(hidden)]
fn main() -> Result<(), ArgError> {
    let (shapes_tx, shapes_rx) = channel::bounded(1);
    let (messages_tx, messages_rx) = channel::unbounded();

    let level = Level::load_from_file(&env::args().nth(1).ok_or(ArgError::MissingFileName)?)?;

    let game_state = GameState(GameStateProperties {
        mouse_position: [1.5, 1.5],
        line_points: vec![[0.0, 0.0], [0.0, 0.0]],
        static_circle: geometry::Circle  {
            center: Point(1.5, 1.5),
            radius: 0.,
        },
        is_beginning_draw: true,
        is_mouse_clicked: false,
        is_holding: false,
        timer: Instant::now(),
        tool: Tool::Crayon,
    });

    let physics = thread::spawn(move || {
        let mut physics = physics::Engine::new(shapes_tx, level);
        loop {
            match messages_rx.try_recv() {
                Ok(InputMessage::Rigid(point)) => physics.add_rigid(point),
                Ok(InputMessage::Erase(point)) => physics.erase_at(point),
                Ok(InputMessage::Hinge(point)) => physics.add_hinge(point),
                Ok(InputMessage::DrawPolygon(vertices)) => {
                    physics.add_polygon(compute::hull::<24>(
                        vertices
                            .into_iter()
                            .map(|[x, y]| Point(x as f64, -y as f64)),
                    ))
                }
                Ok(InputMessage::DrawCircle(geometry::Circle { center, radius })) => {
                    physics.add_circle(Circle::new(center, radius))
                }
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => {}
            }

            physics.run_iteration();
        }
    });

    thread::sleep(Duration::from_millis(100));
    graphics_engine::run(shapes_rx, messages_tx, game_state);
    physics.join().unwrap();
    Ok(())
}
