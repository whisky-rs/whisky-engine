use crossbeam::channel::{self, TryRecvError};
use game_logic::{EditorState, GameState};
use geometry::Point;
use levels::{Entity, Level, LoadError};
use std::{
    env, thread,
    time::{Duration, Instant},
};

use physics::{compute, shape::Circle};

pub mod game_logic;
pub mod geometry;
pub mod graphics_engine;
pub mod levels;
pub mod physics;

pub enum InputMessage {
    Erase(Point),
    Rigid(Point),
    Hinge(Point),
    DrawPolygon(Vec<[f32; 2]>),
    DrawCircle(geometry::Circle),
    Angle(f32),
    Jump,
    CreateLevelShape([f32; 2], [f32; 2], EditorState),
    RemoveLastShape,
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

    let mut level = Level::load_from_file(&env::args().nth(1).ok_or(ArgError::MissingFileName)?)?;

    let game_state = GameState {
        mouse_position: [1.5, 1.5],
        player: geometry::Circle {
            center: Point(1.5, 1.5),
            radius: 0.,
        },
        timer: Instant::now(),
        reset_position: false,
        angle: 0.,
    };

    let physics = thread::spawn(move || {
        loop {
            let mut physics = physics::Engine::new(shapes_tx.clone(), level.clone());
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
                    Ok(InputMessage::CreateLevelShape(p1, p2, ed)) => {
                        level.polygons.push(Entity {
                            shape: vec![
                                Point(p1[0].into(), p1[1].into()),
                                Point(p1[0].into(), p2[1].into()),
                                Point(p2[0].into(), p2[1].into()),
                                Point(p2[0].into(), p1[1].into()),
                            ],
                            is_static: true,
                            is_bindable: false,
                            is_deadly: ed.is_deadly,
                            is_fragile: ed.is_fragile,
                        });
                        level.save_to_file("edited.ron");
                        break;
                    }
                    Ok(InputMessage::DrawCircle(geometry::Circle { center, radius })) => {
                        physics.add_circle(Circle::new(center, radius))
                    }
                    Ok(InputMessage::Angle(angle)) => { /* TODO JEREMI */ }
                    Ok(InputMessage::Jump) => { /* TODO JEREMI */ }
                    Ok(InputMessage::RemoveLastShape) => {
                        level.polygons.pop();
                        level.save_to_file("edited.ron");
                        break;
                    }
                    Err(TryRecvError::Disconnected) => return,
                    Err(TryRecvError::Empty) => {}
                }
            }

            physics.run_iteration();
        }
    });

    let lvledit = thread::spawn(move || {});

    thread::sleep(Duration::from_millis(100));
    graphics_engine::run(shapes_rx, messages_tx, game_state);
    physics.join().unwrap();
    Ok(())
}
