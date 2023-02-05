use std::time::Duration;

use crate::geometry::{Point, Vector};

use super::{compute::simplex::Vertex, shape::Collidable};

/// Refers to a point on a shape. The shape may be translated or rotated
/// without invalidating this reference, since the reference refers to
/// the point relative to center and the first vertex
#[derive(Clone, Copy, Debug)]
pub struct PointOnShape {
    pub angle_offset: f64,
    pub length_scale: f64,
}

impl PointOnShape {
    pub fn on(self, shape: &(impl Collidable + ?Sized)) -> Point {
        shape.resolve_point_reference(self)
    }
}

#[derive(Clone, Copy)]
pub enum Binding {
    Hinge {
        first: PointOnShape,
        second: PointOnShape,
    },
    Rigid {
        first: (PointOnShape, PointOnShape),
        second: (PointOnShape, PointOnShape),
    },
}

impl Binding {
    /// attempts to bind the two shapes together
    /// it is assumed that the unbound binding is attached to the first shape
    pub fn try_bind(
        shape1: &(impl Collidable + ?Sized),
        unbound: Unbound,
        shape2: &(impl Collidable + ?Sized),
    ) -> Option<Self> {
        match unbound {
            Unbound::Hinge(first) => {
                let point = shape1.resolve_point_reference(first);
                if !shape2.includes(point) {
                    return None;
                }

                let second = shape2.create_point_reference(point);

                Some(Self::Hinge { first, second })
            }
            Unbound::Rigid(first) => {
                let point = shape1.resolve_point_reference(first);
                if !shape2.includes(point) {
                    return None;
                }

                let first_left = shape1.create_point_reference(point + Point(0.2, 0.0));
                let first_right = shape1.create_point_reference(point - Point(0.2, 0.0));
                let second_left = shape2.create_point_reference(point + Point(0.2, 0.0));
                let second_right = shape2.create_point_reference(point - Point(0.2, 0.0));

                Some(Self::Rigid {
                    first: (first_left, first_right),
                    second: (second_left, second_right),
                })
            }
        }
    }

    /// enforces the spacial constraints of this binding
    pub fn enforce(
        self,
        shape1: &mut dyn Collidable,
        shape2: &mut dyn Collidable,
        time_step: Duration,
    ) {
        match self {
            Self::Hinge { first, second } => {
                Self::enforce_hinge((shape1, first), (shape2, second), time_step)
            }
            Self::Rigid { first, second } => {
                Self::enforce_hinge((shape1, first.0), (shape2, second.0), time_step);
                Self::enforce_hinge((shape1, first.1), (shape2, second.1), time_step);
            }
        }
    }

    fn enforce_hinge(
        first: (&mut dyn Collidable, PointOnShape),
        second: (&mut dyn Collidable, PointOnShape),
        time_step: Duration,
    ) {
        let point1 = first.1.on(first.0);
        let point2 = second.1.on(second.0);
        let translation = point2.to(point1);
        if !translation.is_close_enough_to(Vector::ZERO) {
            first.0.resolve_collision_with(
                second.0,
                Vertex {
                    point: translation,
                    created_from: (point1, point2),
                },
                time_step,
            );
        }
    }
}

#[derive(Clone, Copy)]
pub enum Unbound {
    Hinge(PointOnShape),
    Rigid(PointOnShape),
}

impl Unbound {
    pub fn new_hinge(shape: &(impl Collidable + ?Sized), at: Point) -> Self {
        Self::Hinge(shape.create_point_reference(at))
    }

    pub fn new_rigid(shape: &(impl Collidable + ?Sized), at: Point) -> Self {
        Self::Rigid(shape.create_point_reference(at))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::physics::make_shape;

    #[test]
    fn test_binding() {
        let shape = make_shape! {
            (0.0, 0.0),
            (1.0, 0.0),
            (1.0, 1.0),
            (0.0, 1.0),
        };

        let unbound = Unbound::new_hinge(&shape, Point(0.9, 0.9));

        assert!(Binding::try_bind(
            &shape,
            unbound,
            &make_shape! {
                (0.8, 0.8),
                (1.8, 0.8),
                (1.8, 1.8),
                (0.8, 1.8),
            }
        )
        .is_some());

        assert!(Binding::try_bind(
            &shape,
            unbound,
            &make_shape! {
                (1.1, 1.1),
                (2.1, 1.1),
                (2.1, 2.1),
                (1.1, 2.1),
            }
        )
        .is_none());
    }
}
