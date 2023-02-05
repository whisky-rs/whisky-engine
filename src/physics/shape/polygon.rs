use crate::{
    geometry::{self, windows, Point, Vector},
    physics::{binding::PointOnShape, compute},
};

use super::{Bounded, Collidable, CollisionData};

#[derive(Clone)]
pub struct Polygon {
    vertices: Vec<Point>,
    collision_properties: CollisionData,
    angle: f64,
}

impl Polygon {
    pub fn new(vertices: Vec<Point>) -> Self {
        let centroid = compute::centroid(&vertices);
        let (inertia, mass) = Self::intertia_and_mass(centroid, &vertices);

        Self {
            vertices,
            collision_properties: CollisionData {
                mass,
                inertia,
                velocity: Vector::ZERO,
                angular_velocity: 0.0,
                centroid,
            },
            angle: 0.0,
        }
    }

    fn intertia_and_mass(centroid: Point, vertices: &[Point]) -> (f64, f64) {
        let centroid_norm_squared = centroid.dot(centroid);
        let (inertia_sum, mass_sum) = windows::Looped::from(
            vertices
                .iter()
                .cloned()
                .map(|point| (point, centroid.to(point), point.dot(point), centroid.dot(point))),
        )
        .map(
            |[
                (point1, from_center1, point_squared1, point_dot_center1),
                (point2, from_center2, point_squared2, point_dot_center2),
            ]| {
                let doubled_mass = from_center1.cross(from_center2);
                (
                    (3.0 * centroid_norm_squared + point_squared1 + point_squared2 + point1.dot(point2)
                        - 3.0 * point_dot_center1
                        - 3.0 * point_dot_center2)
                        * doubled_mass,
                    doubled_mass,
                )
            },
        )
        .reduce(|(inertia_sum, mass_sum), (inertia, mass)| (inertia_sum + inertia, mass_sum + mass))
        .unwrap();
        ((inertia_sum / 12.0).abs(), (mass_sum / 2.0).abs())
    }
}

impl Bounded for Polygon {
    fn support_vector(&self, direction: Vector) -> Vector {
        *self
            .vertices
            .iter()
            .max_by(|&&p1, &&p2| direction.dot(p1).partial_cmp(&direction.dot(p2)).unwrap())
            .unwrap()
    }

    fn includes(&self, point: Point) -> bool {
        let mut last = 0.0;
        for [p1, p2] in windows::Looped::from(self.vertices.iter().copied()) {
            let next = p1.to(p2).perpendicular().dot(p1.to(point));
            if last * next < 0.0 {
                return false;
            }

            last = next;
        }
        true
    }
}

impl Collidable for Polygon {
    fn rotate(&mut self, angle: f64) {
        self.vertices.iter_mut().for_each(|v| {
            let offset = self.collision_properties.centroid.to(*v);
            *v = offset.rotate(angle) + self.collision_properties.centroid;
        });

        self.angle += angle;
    }

    fn translate(&mut self, translation: Vector) {
        self.vertices.iter_mut().for_each(|v| *v += translation);
        self.collision_properties.centroid += translation;
    }

    fn collision_data_mut(&mut self) -> &mut CollisionData {
        &mut self.collision_properties
    }

    fn resolve_point_reference(&self, point_ref: PointOnShape) -> Point {
        (self
            .collision_properties
            .centroid
            .to(self.vertices[0])
            .rotate(point_ref.angle_offset)
            * point_ref.length_scale)
            + self.collision_properties.centroid
    }

    fn create_point_reference(&self, point: Point) -> PointOnShape {
        let to_first_vertex = self.collision_properties.centroid.to(self.vertices[0]);
        let to_point = self.collision_properties.centroid.to(point);
        PointOnShape {
            angle_offset: to_first_vertex.angle_to(to_point),
            length_scale: to_point.norm() / to_first_vertex.norm(),
        }
    }
}

impl From<Polygon> for geometry::Polygon {
    fn from(shape: Polygon) -> Self {
        Self {
            vertices: shape.vertices,
            centroid: shape.collision_properties.centroid,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_includes() {
        let polygon = Polygon::new(vec![
            Point(0.1, 0.3),
            Point(0.3, 0.3),
            Point(0.3, 0.5),
            Point(0.1, 0.5),
        ]);

        assert!(polygon.includes(Point(0.2, 0.4)));
        assert!(!polygon.includes(Point(0.2, 0.6)));
    }
}
