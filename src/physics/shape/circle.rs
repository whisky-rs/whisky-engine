use crate::{
    geometry::{self, Point, Vector},
    physics::binding::PointOnShape,
};

use super::{Bounded, Collidable, CollisionData, Shape};

impl Shape for Circle {
    type Underlying = geometry::Circle;
}

#[derive(Clone)]
pub struct Circle {
    radius: f64,
    angle: f64,
    collision_properties: CollisionData,
}

impl Circle {
    pub fn new(center: Point, radius: f64) -> Self {
        let mass = std::f64::consts::PI * radius.powi(2);
        Self {
            radius,
            angle: 0.0,
            collision_properties: CollisionData {
                centroid: center,
                mass,
                inertia: mass * radius.powi(2) / 2.0,
                velocity: Point::ZERO,
                angular_velocity: 0.0,
            },
        }
    }
}

impl Bounded for Circle {
    fn support_vector(&self, direction: Vector) -> Vector {
        direction.unit() * self.radius + self.collision_properties.centroid
    }

    fn includes(&self, point: Point) -> bool {
        self.collision_properties.centroid.to(point).norm() <= self.radius
    }
}

impl Collidable for Circle {
    fn collision_data_mut(&mut self) -> &mut CollisionData {
        &mut self.collision_properties
    }

    fn translate(&mut self, translation: Vector) {
        self.collision_properties.centroid += translation;
    }

    fn rotate(&mut self, angle: f64) {
        self.angle += angle;
    }

    fn resolve_point_reference(&self, point_ref: PointOnShape) -> Point {
        (Point(self.radius, 0.0).rotate(point_ref.angle_offset + self.angle)
            * point_ref.length_scale)
            + self.collision_properties.centroid
    }

    fn create_point_reference(&self, point: Point) -> PointOnShape {
        let to_point = self.collision_properties.centroid.to(point);
        PointOnShape {
            angle_offset: Point(1.0, 0.0).rotate(self.angle).angle_to(to_point),
            length_scale: to_point.norm() / self.radius,
        }
    }
}

impl From<Circle> for geometry::Circle {
    fn from(circle: Circle) -> Self {
        Self {
            center: circle.collision_properties.centroid,
            radius: circle.radius,
        }
    }
}
