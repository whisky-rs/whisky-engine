pub mod gjk {
    use std::collections::BinaryHeap;

    use super::super::simplex::{self, Simplex};
    use crate::{
        geometry::Point,
        physics::{compute::minkowski, shape::Bounded},
    };

    /// 2D (GJK algorithm)[https://en.wikipedia.org/wiki/Gilbert%E2%80%93Johnson%E2%80%93Keerthi_distance_algorithm]
    ///
    /// Checks for a collision between to shapes by sampling their minkowski difference.
    /// If the samples form a simplex that encloses the origin, the two shapes collide and
    /// the enclosing edges are returned.
    pub fn eclosing_simplex(
        initial_point: Point,
        difference: minkowski::Difference<(impl Bounded + ?Sized), (impl Bounded + ?Sized)>,
    ) -> Option<BinaryHeap<simplex::Edge>> {
        const MAX_ITERATION_COUNT: usize = 40;

        let inital_point = difference.support_vector(initial_point);
        let mut simplex = simplex::Partial::Point(inital_point);
        let mut search_direction = -inital_point.point;
        let mut iteration_count = 0;

        Some(loop {
            match simplex.try_to_enclose(difference.support_vector(search_direction)) {
                simplex::ClosureResult::NextDirection(direction) => {
                    search_direction = direction;
                    if iteration_count > MAX_ITERATION_COUNT {
                        return None;
                    }
                }
                simplex::ClosureResult::ExcludesOrigin => return None,
                simplex::ClosureResult::IncludesOrigin(Simplex::Triangle(first, second, third)) => {
                    break BinaryHeap::from([
                        simplex::Edge::try_new(first, second)?,
                        simplex::Edge::try_new(second, third)?,
                        simplex::Edge::try_new(third, first)?,
                    ]);
                }
                simplex::ClosureResult::IncludesOrigin(Simplex::Line(first, second)) => {
                    let direction = first.point.to(second.point).perpendicular();
                    let third = difference.support_vector(direction);
                    let fourth = difference.support_vector(-direction);

                    break BinaryHeap::from([
                        simplex::Edge::try_new(first, third)?,
                        simplex::Edge::try_new(third, second)?,
                        simplex::Edge::try_new(second, fourth)?,
                        simplex::Edge::try_new(fourth, first)?,
                    ]);
                }
                simplex::ClosureResult::IncludesOrigin(Simplex::Point(_)) => {
                    return None;
                }
            }
            iteration_count += 1;
        })
    }
}

pub mod epa {
    use std::collections::BinaryHeap;

    use super::super::simplex;
    use crate::geometry::EPSILON;
    use crate::physics::shape::Bounded;
    use crate::{geometry::Point, physics::compute::minkowski};

    /// (EPA algorithm)[https://dyn4j.org/2010/05/epa-expanding-polytope-algorithm/]
    ///
    /// Finds the minimum translation vector by iteratively splitting the edge closest to the origin.
    pub fn closest_point_of(
        mut simpex_edges: BinaryHeap<simplex::Edge>,
        difference: minkowski::Difference<(impl Bounded + ?Sized), (impl Bounded + ?Sized)>,
    ) -> simplex::Vertex {
        const MAX_ITERATION_COUNT: usize = 40;

        let mut prev_point = Point(f64::MAX, f64::MAX);
        let mut iteration_count = 0;

        loop {
            let edge = simpex_edges.pop().unwrap();
            let closest_point = edge.towards_segment * edge.distance_to_origin;

            if closest_point.is_close_enough_to(prev_point) || iteration_count > MAX_ITERATION_COUNT
            {
                return try_interpolate(&edge, closest_point, Axis::X)
                    .or_else(|| try_interpolate(&edge, closest_point, Axis::Y))
                    .unwrap_or(edge.segment.0);
            }

            let new_vertex = difference.support_vector(edge.towards_segment);

            simpex_edges.push(simplex::Edge::new(edge.segment.0, new_vertex));
            simpex_edges.push(simplex::Edge::new(new_vertex, edge.segment.1));

            prev_point = closest_point;

            iteration_count += 1;
        }
    }

    enum Axis {
        X,
        Y,
    }

    fn try_interpolate(
        edge: &simplex::Edge,
        closest_point: Point,
        axis: Axis,
    ) -> Option<simplex::Vertex> {
        let (start, middle, end) = match axis {
            Axis::X => (
                edge.segment.0.point.0,
                closest_point.0,
                edge.segment.1.point.0,
            ),
            Axis::Y => (
                edge.segment.0.point.1,
                closest_point.1,
                edge.segment.1.point.1,
            ),
        };

        let distance = end - start;
        if distance.abs() > EPSILON {
            let fact = (middle - start) / distance;
            Some(simplex::Vertex {
                created_from: (
                    edge.segment.0.created_from.0 * (1.0 - fact)
                        + edge.segment.1.created_from.0 * fact,
                    edge.segment.0.created_from.1 * (1.0 - fact)
                        + edge.segment.1.created_from.1 * fact,
                ),
                point: closest_point,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::{super::minkowski, gjk};
    use crate::{geometry::Point, physics::make_shape};

    #[test]
    fn gjk_collides_test() {
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

        let difference = minkowski::Difference(&first, &second);
        assert!(gjk::eclosing_simplex(Point(1.0, 1.0), difference).is_some());
    }

    #[test]
    fn gjk_does_not_collide_test() {
        let first = make_shape! {
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
        };

        let second = make_shape! {
            (3.0, 3.0),
            (5.0, 3.0),
            (5.0, 5.0),
            (3.0, 5.0),
        };

        let difference = minkowski::Difference(&first, &second);
        assert!(gjk::eclosing_simplex(Point(1.0, 1.0), difference).is_none());
    }
}
