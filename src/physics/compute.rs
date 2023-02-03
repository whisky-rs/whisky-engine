use std::{
    f64::consts::PI,
    panic::{self, RefUnwindSafe},
};

use super::shape::{Bounded, CollisionData, Polygon};
use crate::geometry::{windows, Point, Vector};

pub mod algorithm;
pub mod minkowski;
pub mod simplex;

/// returns the minimum translation vector necessary to resolve a collsion
/// between `first` and `second`, or `None` if they are not colliding
pub fn collision(
    first: &(impl Bounded + ?Sized + RefUnwindSafe),
    second: &(impl Bounded + ?Sized + RefUnwindSafe),
) -> Option<simplex::Vertex> {
    // this is here bacause in some very rare cases there appear NaNs in the calculations.
    // The algorithms cannot work with NaNs and panics when attempting to compare them.
    // Since one of the last fixes these panics were not observed, but they might just be
    // very difficult to cause
    panic::catch_unwind(|| {
        let difference = minkowski::Difference(first, second);
        let initial_point = Point(0.0, 1.0);
        let simplex = algorithm::gjk::eclosing_simplex(initial_point, difference)?;

        Some(algorithm::epa::closest_point_of(simplex, difference))
    })
    .ok()
    .flatten()
}

/// computes the impulse resulting from a collision between
/// `first` and `second`. The offsets are vectors from the centers
/// of the shapes to the point of contact between them
pub fn impulse(
    first: CollisionData,
    second: CollisionData,
    first_offset: Vector,
    second_offset: Vector,
    collision_normal: Vector,
    relative_velocity: Vector,
    reflection_factor: f64,
) -> f64 {
    -collision_normal.dot(relative_velocity * reflection_factor)
        / (first.mass.recip() + second.mass.recip()
            - collision_normal.dot(
                (first_offset.triple_product(collision_normal) / first.inertia)
                    + (second_offset.triple_product(collision_normal) / second.inertia),
            ))
}

/// Wikipedia translated to Rust: [centroid of a polygon](https://en.wikipedia.org/wiki/Centroid#Of_a_polygon)
pub fn centroid(vertices: &[Point]) -> Point {
    let (combined_points, doubled_area) = windows::Looped::from(vertices.iter().cloned())
        .map(|[first, second]| (first + second, first.cross(second)))
        .fold(
            (Point::ZERO, 0.0),
            |(points_acc, area_acc), (point, area)| (points_acc + point * area, area_acc + area),
        );

    combined_points / (3.0 * doubled_area)
}

/// wraps an at most `N` vertex hull around the provided collection of vertices
/// I would love to put the `directions` array in a constant, but unfortunately
/// Rust does not support generic const/statics. The static rvalue promotion hack
/// is also not an option here due to the "complex" initalization scheme of the array
///
/// Panics if the iterator is empty
pub fn hull<const N: usize>(mut points: impl Iterator<Item = Point>) -> Polygon {
    let first = points
        .next()
        .expect("cannot create a hull from an empty set of verticies");

    let mut directions = [Vector::ZERO; N];
    let mut maximally_extended_points = [first; N];
    let mut maximally_extended_points_dots = [0.0; N];

    for i in 0..N {
        directions[i] = Point(1.0, 0.0).rotate((2 * i) as f64 * PI / N as f64);
        maximally_extended_points_dots[i] = first.dot(directions[i]);
    }

    for point in points {
        for i in 0..N {
            let new_dot = point.dot(directions[i]);
            if new_dot > maximally_extended_points_dots[i] {
                maximally_extended_points[i] = point;
                maximally_extended_points_dots[i] = new_dot;
            }
        }
    }
    // filter out closely neighbouring vertices before creating the polygon
    Polygon::new(maximally_extended_points.into_iter().fold(
        Vec::<Point>::with_capacity(N),
        |mut vertices, extended_point| match vertices.last() {
            Some(vertex) if !vertex.is_close_enough_to(extended_point) => {
                vertices.push(extended_point);
                vertices
            }
            None => {
                vertices.push(extended_point);
                vertices
            }
            _ => vertices,
        },
    ))
}
