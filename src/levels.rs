use std::{fs, io, path::Path};

use serde::Deserialize;

use crate::geometry::{Circle, Point};

#[derive(Deserialize)]
pub struct Entity<S> {
    pub shape: S,
    pub is_static: bool,
    pub is_bindable: bool,
}

/// Represents a single level
///
/// intended to be loadaed from a file specified by the user in RON notation
/// and passed directly to the physics engine
#[derive(Deserialize)]
pub struct Level {
    pub initial_ball_position: Point,
    pub circles: Vec<Entity<Circle>>,
    pub polygons: Vec<Entity<Vec<Point>>>,
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
}
