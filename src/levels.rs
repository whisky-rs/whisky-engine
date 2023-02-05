use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};
use crate::{geometry::{Circle, Laser, Point}};


fn initialize_false() -> bool {
    false
}

fn initialize_empty_laser() -> Vec<Laser> {
    vec![]
}

fn initialize_empty_door() -> Vec<Vec<Point>> {
    vec![]
}


#[derive(Clone, Deserialize, Serialize)]
pub struct Entity<S> {
    pub shape: S,
    pub is_static: bool,
    pub is_bindable: bool,
    #[serde(default = "initialize_false")]
    pub is_deadly: bool,
    #[serde(default = "initialize_false")]
    pub is_fragile: bool,
}

/// Represents a single level
///
/// intended to be loadaed from a file specified by the user in RON notation
/// and passed directly to the physics engine
#[derive(Clone, Deserialize, Serialize)]
pub struct Level {
    pub initial_ball_position: Point,
    pub circles: Vec<Entity<Circle>>,
    pub polygons: Vec<Entity<Vec<Point>>>,
    #[serde(default = "initialize_empty_laser")]
    pub lasers: Vec<Laser>,
    #[serde(default = "initialize_empty_door")]
    pub doors: Vec<Vec<Point>>,
    pub flags_positions: Vec<Point>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("the specified file is invalid: {0}")]
    Io(#[from] io::Error),
    #[error("there was an error parsing the level: {0}")]
    Parse(#[from] ron::error::SpannedError),
}

impl Level {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Level, LoadError> {
        Ok(ron::from_str(&fs::read_to_string(path)?)?)
    }
    pub fn save_to_file(&self, path: impl AsRef<Path>) {
        fs::write(path, ron::to_string(self).unwrap()).unwrap();
    }
}
