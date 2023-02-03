use crate::{
    geometry::Vector,
    physics::{compute::simplex, shape::Bounded},
};

pub struct Difference<'s, S1: ?Sized, S2: ?Sized>(pub &'s S1, pub &'s S2);

// why the Copy and Clone derive macros place Copy bounds on S1 and S2 here is beyond me
// so this is just their manual expansion without the unnecessary bounds on S1 and S2
impl<'s, S1: ?Sized, S2: ?Sized> Copy for Difference<'s, S1, S2> {}
impl<'s, S1: ?Sized, S2: ?Sized> Clone for Difference<'s, S1, S2> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<'s, S1: Bounded + ?Sized, S2: Bounded + ?Sized> Difference<'s, S1, S2> {
    pub fn support_vector(&self, direction: Vector) -> simplex::Vertex {
        let first = self.0.support_vector(direction);
        let second = self.1.support_vector(-direction);

        simplex::Vertex {
            point: first - second,
            created_from: (first, second),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{geometry::Point, physics::make_shape};

    use super::*;

    #[test]
    fn test_support_vector() {
        let first = make_shape! {
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
        };

        let second = make_shape! {
            (1.0, 1.0),
            (3.0, 1.0),
            (3.0, 3.0),
            (1.0, 3.0),
        };

        let difference = Difference(&first, &second);

        let vertex = difference.support_vector(Point(1.0, 1.0));
        assert!(vertex.point.is_close_enough_to(Point(1.0, 1.0)));

        let vertex = difference.support_vector(Point(-1.0, -1.0));
        assert!(vertex.point.is_close_enough_to(Point(-3.0, -3.0)));
    }
}
