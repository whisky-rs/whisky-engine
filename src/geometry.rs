use std::ops;

use serde::{Deserialize, Serialize};

pub const EPSILON: f64 = 1e-7;

/// A point on the 2D plane or a vector.
///
/// The types of receivers and parameters are mostly specified explicitly
/// as either `Point` or the type alias `Vector`, to suggest the correct intepretation
/// of these values within a given context
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Point(pub f64, pub f64);

impl Point {
    pub const ZERO: Self = Self(0.0, 0.0);

    pub fn dot(self: Vector, other: Vector) -> f64 {
        self.0 * other.0 + self.1 * other.1
    }

    pub fn to(self: Point, other: Point) -> Vector {
        other - self
    }

    pub fn is_close_enough_to(self, other: Self) -> bool {
        (other.0 - self.0).abs() < EPSILON && (other.1 - self.1).abs() < EPSILON
    }

    pub fn cross(self: Vector, other: Vector) -> f64 {
        self.0 * other.1 - self.1 * other.0
    }

    pub fn perpendicular(self: Vector) -> Vector {
        Self(self.1, -self.0)
    }

    pub fn rotate(self: Vector, angle: f64) -> Vector {
        Self(
            self.0 * angle.cos() - self.1 * angle.sin(),
            self.0 * angle.sin() + self.1 * angle.cos(),
        )
    }

    pub fn unit(self: Vector) -> Vector {
        self / self.norm()
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn angle_to(self: Vector, other: Vector) -> f64 {
        (self.unit().dot(other.unit())).min(1.0).acos()
            * if self.cross(other) > 0.0 { 1.0 } else { -1.0 }
    }

    pub fn triple_product(self: Vector, other: Vector) -> Vector {
        let segment = other.to(self);
        -other * segment.dot(segment) - segment * segment.dot(-other)
    }
}

/// Used instead of `Point` to suggest that a point represents a vector,
/// and not a point on the 2D plane
pub type Vector = Point;

impl ops::Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Point(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl ops::Sub for Point {
    type Output = Point;
    fn sub(self, rhs: Self) -> Self::Output {
        Point(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl ops::AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl ops::SubAssign for Point {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl ops::Mul<f64> for Point {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Point(self.0 * rhs, self.1 * rhs)
    }
}

impl ops::Div<f64> for Point {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Point(self.0 / rhs, self.1 / rhs)
    }
}

impl ops::Neg for Point {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Point(-self.0, -self.1)
    }
}

impl From<[f32; 2]> for Point {
    fn from([x, y]: [f32; 2]) -> Self {
        Self(x as f64, y as f64)
    }
}

#[derive(Debug)]
pub struct Polygon {
    pub vertices: Vec<Point>,
    pub centroid: Point,
}

impl Polygon {
    pub fn rotate(&mut self, angle: f32) {
        for vertex in &mut self.vertices {
            *vertex = vertex.rotate(angle as f64);
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

impl Circle {
    pub fn rotate(&mut self, angle: f32) {
        self.center = self.center.rotate(angle as f64);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Laser {
    pub point: Point,
    pub direction: Vector,
    pub change: f64,
}

#[cfg(test)]
mod test {
    use std::f64::consts::PI;

    use super::*;
    #[test]
    fn t() {
        assert!((Point(0.0, 1.0).rotate(PI / 2.0).angle_to(Point(1.0, 0.0)) % PI).abs() < EPSILON);
        assert!(Point(1.0, 0.0)
            .rotate(PI / 2.0)
            .is_close_enough_to(Point(0.0, 1.0)))
    }
}

/// An iterator very much like the standard library [std::slice::Windows], [`std::slice::Windows`],
/// but it wraps around and uses const generics
pub mod windows {
    use std::mem::{self, MaybeUninit};

    pub struct Looped<I: Iterator, const N: usize> {
        items: I,
        state: Option<State<I::Item, N>>,
    }

    struct State<T, const N: usize> {
        first: [T; N],
        next_from_beg_idx: usize,
        previous: [T; N],
    }

    impl<T: Copy, const N: usize> State<T, N> {
        fn new(items: &mut impl Iterator<Item = T>) -> Option<State<T, N>> {
            let mut first: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

            for item in &mut first {
                item.write(items.next()?);
            }

            let first = unsafe { mem::transmute_copy(&first) };

            Some(Self {
                first,
                previous: first,
                next_from_beg_idx: 0,
            })
        }
    }

    impl<I: Iterator, const N: usize> From<I> for Looped<I, N>
    where
        I::Item: Copy,
    {
        fn from(items: I) -> Self {
            Looped { items, state: None }
        }
    }

    impl<I: Iterator, const N: usize> Iterator for Looped<I, N>
    where
        I::Item: Copy,
    {
        type Item = [I::Item; N];

        fn next(&mut self) -> Option<Self::Item> {
            Some(match &mut self.state {
                Some(state) => {
                    let next = self.items.next().or_else(|| {
                        if state.next_from_beg_idx >= N - 1 {
                            return None;
                        }

                        let next = state.first[state.next_from_beg_idx];
                        state.next_from_beg_idx += 1;
                        Some(next)
                    })?;

                    state.previous.copy_within(1.., 0);
                    state.previous[N - 1] = next;
                    state.previous
                }
                state @ None => {
                    let new_state = State::new(&mut self.items)?;
                    let previous = new_state.previous;
                    *state = Some(new_state);
                    previous
                }
            })
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_looped() {
            let mut iter: Looped<_, 3> = [1, 2, 3, 4, 5].into_iter().into();

            assert_eq!(iter.next(), Some([1, 2, 3]));
            assert_eq!(iter.next(), Some([2, 3, 4]));
            assert_eq!(iter.next(), Some([3, 4, 5]));
            assert_eq!(iter.next(), Some([4, 5, 1]));
            assert_eq!(iter.next(), Some([5, 1, 2]));
            assert_eq!(iter.next(), None);
        }
    }
}
