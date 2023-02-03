//! this module provides utilities for working with simplices necessary for the GJK and EPA algorithm implementations

use std::cmp::Ordering;

use crate::geometry::Point;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub point: Point,
    pub created_from: (Point, Point),
}

#[derive(Debug)]
pub enum Partial {
    Point(Vertex),
    Line(Vertex, Vertex),
}

pub enum Simplex {
    Point(Vertex),
    Line(Vertex, Vertex),
    Triangle(Vertex, Vertex, Vertex),
}

pub enum ClosureResult {
    NextDirection(Point),
    ExcludesOrigin,
    IncludesOrigin(Simplex),
}

impl Partial {
    pub fn try_to_enclose(&mut self, new: Vertex) -> ClosureResult {
        if new.point.is_close_enough_to(Point::ZERO) {
            return ClosureResult::IncludesOrigin(Simplex::Point(new));
        }

        match self {
            Self::Point(old) if old.point.is_close_enough_to(new.point) => {
                ClosureResult::ExcludesOrigin
            }
            Self::Point(old) if new.point.dot(new.point.to(old.point)) > 0.0 => {
                *old = new;
                ClosureResult::NextDirection(-new.point)
            }
            &mut Self::Point(old) => {
                *self = Self::Line(new, old);

                let dir = old.point.triple_product(new.point);
                if dir == Point::ZERO {
                    return ClosureResult::IncludesOrigin(Simplex::Line(old, new));
                }

                ClosureResult::NextDirection(dir)
            }
            Self::Line(one, two)
                if one.point.is_close_enough_to(new.point)
                    || two.point.is_close_enough_to(new.point) =>
            {
                ClosureResult::ExcludesOrigin
            }
            Self::Line(one, two) => {
                let first_arm = new.point.to(one.point);
                let second_arm = new.point.to(two.point);
                match (
                    new.point.dot(first_arm) > 0.0,
                    new.point.dot(second_arm) > 0.0,
                ) {
                    (true, true) => {
                        *self = Self::Point(new);
                        ClosureResult::NextDirection(-new.point)
                    }
                    (false, false) => {
                        let first_cross = new.point.cross(first_arm);
                        let second_cross = new.point.cross(second_arm);
                        if first_cross * second_cross < 0.0 {
                            ClosureResult::IncludesOrigin(Simplex::Triangle(*one, *two, new))
                        } else {
                            let (redundant, other) = if first_cross.abs() > second_cross.abs() {
                                (one, two)
                            } else {
                                (two, one)
                            };

                            *redundant = new;
                            ClosureResult::NextDirection(other.point.triple_product(new.point))
                        }
                    }
                    (first_redundant, _) => {
                        let (redundant, other) = if first_redundant {
                            (one, two)
                        } else {
                            (two, one)
                        };

                        *redundant = new;
                        ClosureResult::NextDirection(other.point.triple_product(new.point))
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Edge {
    pub distance_to_origin: f64,
    pub towards_segment: Point,
    pub segment: (Vertex, Vertex),
}

impl Edge {
    pub fn new(first: Vertex, second: Vertex) -> Self {
        Self::try_new(first, second).unwrap()
    }

    pub fn try_new(first: Vertex, second: Vertex) -> Option<Self> {
        if first.point.to(second.point).dot(-first.point) <= 0.0 {
            return Some(Self::redundant(first, second));
        }

        if second.point.to(first.point).dot(-second.point) <= 0.0 {
            return Some(Self::redundant(second, first));
        }

        let to_origin = first.point.triple_product(second.point).unit();
        let distance_to_origin = -first.point.dot(to_origin);

        if distance_to_origin.is_nan() {
            None
        } else {
            Some(Self {
                distance_to_origin,
                towards_segment: -to_origin,
                segment: (first, second),
            })
        }
    }

    fn redundant(primary: Vertex, redundant: Vertex) -> Self {
        Self {
            distance_to_origin: primary.point.dot(primary.point).sqrt(),
            towards_segment: primary.point,
            segment: (primary, redundant),
        }
    }
}

impl Eq for Edge {}
impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        self.distance_to_origin.eq(&other.distance_to_origin)
    }
}

impl PartialOrd for Edge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.distance_to_origin
            .partial_cmp(&other.distance_to_origin)
            .map(Ordering::reverse)
    }
}

impl Ord for Edge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic]
    fn test_edge() {
        let first = Vertex {
            point: Point(1.0, 1.0),
            created_from: (Point(1.0, 1.0), Point::ZERO),
        };

        let second = Vertex {
            point: Point(-1.0, -1.0),
            created_from: (Point(-1.0, -1.0), Point::ZERO),
        };

        Edge::new(first, second);
    }
}
