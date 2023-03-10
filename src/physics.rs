use std::{
    cell::RefCell,
    f64::consts,
    rc::{Rc, Weak},
    time::Instant,
    vec, f32::consts::E,
};

use crossbeam::channel::{self, TrySendError};
use rand::Rng;

use self::{
    binding::{Binding, Unbound},
    shape::{Bounded, Circle, Collidable, CollisionType, Polygon},
};
use crate::{
    geometry::{self, Laser, Point, Vector},
    levels::Level,
};

mod binding;
pub mod compute;
pub mod shape;

const GRAVITY_COEFFICIENT: f64 = -0.000002;
const MOVEMENT_COEFFICIENT: f64 = 0.0000004;

#[derive(Debug)]
pub struct WithColor<S> {
    pub color: [f32; 3],
    pub shape: S,
}

impl<S> From<S> for WithColor<S> {
    fn from(shape: S) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            color: [
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
            ],
            shape,
        }
    }
}

pub struct DisplayMessage {
    pub polygons: Vec<WithColor<geometry::Polygon>>,
    pub circles: Vec<WithColor<geometry::Circle>>,
    pub flags: Vec<geometry::Polygon>,
    pub rigid_bindings: Vec<geometry::Point>,
    pub hinges: Vec<Point>,
    pub unbound_rigid_bindings: Vec<Point>,
    pub unbound_hinges: Vec<Point>,
    pub lasers: Vec<WithColor<geometry::Polygon>>,
    pub laser_boxes: Vec<WithColor<geometry::Polygon>>,
    pub doors: Vec<WithColor<geometry::Polygon>>,
    pub level_idx: usize,
}

fn to_geometry<G>(
    shapes: &mut Vec<WithColor<Weak<RefCell<impl Into<G> + Clone>>>>,
) -> Vec<WithColor<G>> {
    let mut geometry_shapes = Vec::with_capacity(shapes.len());
    shapes.retain(|colored_shape| {
        if let Some(shape) = colored_shape.shape.upgrade() {
            geometry_shapes.push(WithColor {
                color: colored_shape.color,
                shape: shape.borrow().clone().into(),
            });
            true
        } else {
            false
        }
    });
    geometry_shapes
}

fn polygon_to_geometry(
    polygons: Vec<Polygon>,
    color: [f32; 3],
) -> Vec<WithColor<geometry::Polygon>> {
    let mut geometry_shapes = Vec::with_capacity(polygons.len());
    for laser in polygons.iter() {
        let colored_laser = WithColor {
            shape: laser,
            color,
        };
        geometry_shapes.push(WithColor {
            color: colored_laser.color,
            shape: laser.clone().into(),
        });
    }
    geometry_shapes
}

#[cfg(test)]
macro_rules! make_shape {
    ($(($x:expr, $y:expr)),*$(,)?) => {
        $crate::physics::shape::Polygon::new(vec![
            $($crate::geometry::Point($x, $y)),*
        ])
    };
}

#[cfg(test)]
pub(crate) use make_shape;

struct EntityCfg {
    is_erasable: bool,
    is_bindable: bool,
    is_static: bool,
    is_deadly: bool,
    is_fragile: bool,
}

impl Default for EntityCfg {
    fn default() -> Self {
        EntityCfg {
            is_erasable: true,
            is_bindable: true,
            is_static: false,
            is_deadly: false,
            is_fragile: false,
        }
    }
}

struct Entity {
    bindings: Vec<(Binding, Weak<RefCell<dyn Collidable>>)>,
    unbound: Vec<Unbound>,
    is_erasable: bool,
    is_bindable: bool,
    is_static: bool,
    is_deadly: bool,
    is_fragile: bool,
    shape: Rc<RefCell<dyn Collidable>>,
}

impl Entity {
    fn new(shape: Rc<RefCell<dyn Collidable>>, entity_type: EntityCfg) -> Self {
        let EntityCfg {
            is_erasable,
            is_bindable,
            is_static,
            is_deadly,
            is_fragile,
        } = entity_type;

        Self {
            bindings: vec![],
            unbound: vec![],
            shape,
            is_static,
            is_erasable,
            is_bindable,
            is_deadly,
            is_fragile,
        }
    }

    fn add_rigid(&mut self, at: Point) {
        self.unbound
            .push(Unbound::new_rigid(&*self.shape.borrow(), at))
    }

    fn add_hinge(&mut self, at: Point) {
        self.unbound
            .push(Unbound::new_hinge(&*self.shape.borrow(), at))
    }

    fn try_bind(&mut self, target: &Rc<RefCell<dyn Collidable>>) {
        self.unbound.retain(|unbound| {
            if let Some(binding) =
                Binding::try_bind(&*self.shape.borrow_mut(), *unbound, &*target.borrow_mut())
            {
                self.bindings.push((binding, Rc::downgrade(target)));
                false
            } else {
                true
            }
        })
    }
}

pub struct Engine {
    channel: channel::Sender<DisplayMessage>,
    // each entity may contain bidings with pointers to entities
    // ocurring later in the vector
    entities: Vec<Entity>,
    // circles and polygons kept separate on the side,
    // because that's how they need to be passed to the graphics.
    // The Rc<RefCell<_>> is pretty much unavoidable,
    // mostly because shapes need to be accessed both via the main vector of entities
    // as well as through bindings. If bindings stored indexes into the vector rather than
    // weak pointers then they would have to be manually updated after removing an entity
    polygons: Vec<WithColor<Weak<RefCell<Polygon>>>>,
    circles: Vec<WithColor<Weak<RefCell<Circle>>>>,
    lasers: Vec<Laser>,
    doors: Vec<(Polygon, String)>,
    laser_boxes: Vec<Polygon>,
    main_ball_starting_position: Point,
    flags: Vec<Polygon>,
    last_iteration: Instant,
    main_ball: Weak<RefCell<Circle>>,
    pub angle: f32,
    jumps_count: usize,
    pub next_level: Option<String>,
    level_stack: Vec<String>,
}

impl Engine {
    pub fn new(
        channel: channel::Sender<DisplayMessage>,
        Level {
            initial_ball_position,
            circles,
            polygons,
            lasers,
            doors,
            flags_positions,
        }: Level,
    ) -> Self {
        let n_of_circles = circles.len() + 1;
        let n_of_polygons = polygons.len();
        let n_of_laser_boxes = lasers.len();

        let doors = doors
            .into_iter()
            .map(|temp_door| (Polygon::new(temp_door.0), temp_door.1))
            .collect();

        let mut engine = Self {
            channel,
            entities: Vec::with_capacity(n_of_circles + n_of_polygons),
            circles: Vec::with_capacity(n_of_circles),
            polygons: Vec::with_capacity(n_of_polygons),
            main_ball_starting_position: initial_ball_position,
            flags: flags_positions
                .into_iter()
                .map(|Point(x, y)| {
                    Polygon::new(vec![
                        geometry::Point(x, y),
                        geometry::Point(x + 0.1, y),
                        geometry::Point(x + 0.1, y + 0.1),
                        geometry::Point(x, y + 0.1),
                    ])
                })
                .collect(),
            last_iteration: Instant::now(),
            main_ball: Weak::new(),
            angle: 0.0,
            lasers,
            laser_boxes: Vec::with_capacity(n_of_laser_boxes),
            doors,
            jumps_count: 2,
            next_level: None,
            level_stack: vec!["level5.ron".to_string()],
        };

        let main_ball_weak = engine.add_entity(
            Circle::new(initial_ball_position, 0.07),
            EntityCfg {
                is_bindable: false,
                is_erasable: false,
                is_static: false,
                is_deadly: false,
                is_fragile: false,
            },
        );

        engine.main_ball = main_ball_weak.clone();

        engine.circles.push(main_ball_weak.into());

        for entity in polygons {
            let weak = engine.add_entity(
                Polygon::new(entity.shape),
                EntityCfg {
                    is_bindable: entity.is_bindable,
                    is_static: entity.is_static,
                    is_erasable: false,
                    is_deadly: entity.is_deadly,
                    is_fragile: entity.is_fragile,
                },
            );
            engine.polygons.push(WithColor {
                color: if !entity.is_static {
                    [1.0, 0.85, 0.22]
                } else if entity.is_deadly {
                    [1.0, 0.0, 0.0]
                } else if entity.is_fragile {
                    [0.7, 0.7, 0.7]
                } else {
                    [1.0, 0.85, 0.42]
                },
                shape: weak,
            })
        }

        for entity in circles {
            let geometry::Circle { center, radius } = entity.shape;
            let weak = engine.add_entity(
                Circle::new(center, radius),
                EntityCfg {
                    is_bindable: entity.is_bindable,
                    is_static: entity.is_static,
                    is_erasable: false,
                    is_deadly: entity.is_deadly,
                    is_fragile: entity.is_fragile,
                },
            );
            engine.circles.push(WithColor {
                color: if !entity.is_static {
                    [1.0, 0.85, 0.22]
                } else if entity.is_deadly {
                    [1.0, 0.0, 0.0]
                } else if entity.is_fragile {
                    [0.7, 0.7, 0.7]
                } else {
                    [1.0, 0.85, 0.42]
                },
                shape: weak,
            });
        }

        engine
    }

    pub fn run_iteration(&mut self) {
        let time_step = self.last_iteration.elapsed();
        let mut is_reset_level = false;
        let mut is_reset_jumps = false;
        self.last_iteration = Instant::now();

        // move all shapes, removing ones out of bounds
        // don't remove the first one though, as it's the main ball
        let mut is_main_ball = true;
        self.entities.retain_mut(|entity| {
            let mut shape = entity.shape.borrow_mut();

            if !entity.is_static {
                shape.update_position(time_step, -self.angle as f64);
            }

            let retain = shape.collision_data_mut().centroid.1 > -5.0 || is_main_ball;
            is_main_ball = false;
            retain
        });

        for door in &self.doors {
            if compute::collision(&door.0, &*self.main_ball.upgrade().unwrap().borrow()).is_some() {
                self.next_level = Some(door.1.clone());
                break;
            }
        }

        //  generate laser polygons
        let mut laser_polygons: Vec<Polygon> = Vec::with_capacity(self.lasers.len());
        for laser in self.lasers.iter() {
            let start_point = laser.point;
            let delta = laser.direction * 0.1;
            let mut end_point = start_point + delta;
            loop {
                let main_ball_rc = self.main_ball.upgrade().unwrap();
                if main_ball_rc.borrow().includes(end_point) {
                    is_reset_level = true;
                    break;
                }
                let result = self
                    .entities
                    .iter()
                    .any(|entity| entity.shape.borrow().includes(end_point));
                if result {
                    let offset = laser.direction.perpendicular().unit() * 0.02;
                    let start_point_second = start_point + offset;
                    let end_point_second = end_point + offset;
                    laser_polygons.push(Polygon::new(vec![
                        start_point,
                        end_point,
                        end_point_second,
                        start_point_second,
                    ]));
                    break;
                }
                end_point += delta;
            }
        }

        // generate laser boxes
        let mut laser_boxes: Vec<Polygon> = Vec::with_capacity(self.lasers.len());
        for laser in self.lasers.iter() {
            let center = laser.point;
            let x_offset = Point(0.03, 0.);
            let y_offset = Point(0., 0.03);
            let first = center - x_offset - y_offset;
            let second = center - x_offset + y_offset;
            let third = center + x_offset + y_offset;
            let fourth = center + x_offset - y_offset;
            laser_boxes.push(Polygon::new(vec![first, second, third, fourth]));
        }
        self.laser_boxes = laser_boxes;

        // return main ball to starting point if out of bounds
        // and check win condition
        {
            let mut ball = self.entities[0].shape.borrow_mut();
            let data = ball.collision_data_mut();

            if data.centroid.0.abs() > 5.0 || data.centroid.1 < -5.0 {
                is_reset_level = true;
            }
        }

        // iterate over all pairs of shapes
        {
            let mut i = 0;
            let mut to_remove = vec![];

            while let [this, rest @ ..] = &mut self.entities[i..] {
                let mut shape = this.shape.borrow_mut();
                if shape.collision_data_mut().inertia < 0.0 || shape.collision_data_mut().mass < 0.0
                {
                    println!("Fuck {i}");
                }
                // collide them if they are not bound
                rest.iter_mut().enumerate().for_each(|(j, other)| {
                    if this.is_static && other.is_static {
                        return;
                    }
                    // let mut is_boud_to_other = false;
                    // this.bindings.retain(|(_, target)| {
                    //     let valid = target.strong_count() > 0;
                    //     if valid {
                    //         is_boud_to_other = is_boud_to_other
                    //             || std::ptr::eq(
                    //                 target.as_ptr() as *const c_void,
                    //                 (&*other.shape) as *const _ as *const c_void,
                    //             )
                    //     }
                    //     valid
                    // });

                    // if !is_boud_to_other {
                    let collision = shape.collide(&mut *other.shape.borrow_mut(), time_step);
                    if let CollisionType::Strong = collision {
                        if this.is_fragile {
                            to_remove.push(i);
                        }
                        if other.is_fragile {
                            to_remove.push(i + j + 1);
                        }
                    }

                    if let (0, CollisionType::Weak | CollisionType::Strong) = (i, collision) {
                        if other.is_deadly {
                            is_reset_level = true;
                        } else {
                            is_reset_jumps = true;
                        }
                    }
                    //     if let CollisionType::Weak | CollisionType::Strong = collision {
                    //         self.next_level = Some("level3.ron".to_string());
                    //         // println!("=========== OOF ==========");
                    //         // process::exit(0);
                    //     }
                    // }
                    // }
                });

                // enforce binding constraints
                this.bindings.iter().for_each(|(binding, target)| {
                    if let Some(other) = target.upgrade() {
                        binding.enforce(&mut *shape, &mut *other.borrow_mut(), time_step)
                    }
                });

                i += 1;
            }
            to_remove.dedup();
            to_remove.sort();
            for i in to_remove.into_iter().rev() {
                let _ = &self.entities.remove(i);
            }
        }

        if self.channel.is_empty() {
            self.prune_and_send_shapes(laser_polygons);
        }

        if is_reset_level {
            if self.level_stack.len() > 1 {
                self.level_stack.pop();
                self.next_level = Some(self.level_stack.last().unwrap().clone());
            } else {
                self.reset_level();
            }
        }

        if is_reset_jumps {
            self.reset_jumps();
        }
    }

    fn prune_and_send_shapes(&mut self, laser_polygons: Vec<Polygon>) {
        let mut rigid_bindings = Vec::new();
        let mut hinges = Vec::new();
        let mut unbound_rigid_bindings = Vec::new();
        let mut unbound_hinges = Vec::new();

        for Entity {
            bindings,
            unbound,
            shape,
            ..
        } in &self.entities
        {
            for (binding, _) in bindings {
                match binding {
                    Binding::Hinge { first, .. } => hinges.push(first.on(&*shape.borrow())),
                    Binding::Rigid {
                        first: (p1, p2), ..
                    } => {
                        let shape = shape.borrow();
                        rigid_bindings.push((p1.on(&*shape) + p2.on(&*shape)) * 0.5)
                    }
                }
            }

            for binding in unbound {
                match binding {
                    Unbound::Hinge(point) => unbound_hinges.push(point.on(&*shape.borrow())),
                    Unbound::Rigid(point) => {
                        unbound_rigid_bindings.push(point.on(&*shape.borrow()))
                    }
                }
            }
        }

        let mut polygons: Vec<WithColor<geometry::Polygon>> = to_geometry(&mut self.polygons);
        let mut circles: Vec<WithColor<geometry::Circle>> = to_geometry(&mut self.circles);

        let mut lasers: Vec<WithColor<geometry::Polygon>> =
            Vec::with_capacity(laser_polygons.len());
        let mut laser_boxes: Vec<WithColor<geometry::Polygon>> =
            Vec::with_capacity(self.laser_boxes.len());
        let mut doors: Vec<WithColor<geometry::Polygon>> = Vec::with_capacity(self.doors.len());

        for laser in polygon_to_geometry(laser_polygons, [0.0, 0.0, 1.0]) {
            lasers.push(laser);
        }

        for laser_box in polygon_to_geometry(self.laser_boxes.clone(), [0.0, 0.0, 1.0]) {
            laser_boxes.push(laser_box);
        }

        for door in polygon_to_geometry(
            self.doors.iter().map(|(d, _)| d.clone()).collect(),
            [0.0, 1.0, 0.0],
        ) {
            doors.push(door);
        }

        for polygon in &mut polygons {
            polygon.shape.rotate(self.angle);
        }

        for circle in &mut circles {
            circle.shape.rotate(self.angle);
        }

        for circle in &mut lasers {
            circle.shape.rotate(self.angle);
        }

        for circle in &mut laser_boxes {
            circle.shape.rotate(self.angle);
        }

        for circle in &mut doors {
            circle.shape.rotate(self.angle);
        }

        if let Err(TrySendError::Disconnected(_)) = self.channel.try_send(DisplayMessage {
            polygons,
            circles,
            flags: self.flags.iter().cloned().map(Into::into).collect(),
            rigid_bindings,
            hinges,
            unbound_rigid_bindings,
            unbound_hinges,
            lasers,
            laser_boxes,
            doors,
            level_idx: self.level_stack.last().unwrap().trim_start_matches("level")[..1]
                .parse()
                .unwrap(),
        }) {
            panic!("failed to send");
        }
        for laser in &mut self.lasers {
            if (Vector::angle_to(laser.inital_direction, laser.direction)).abs() >= laser.range && !laser.is_out {
                laser.is_out = true;
                laser.change *= -1.;
            } else {
                laser.is_out = false;
            }
            // println!("{}", Vector::angle_to(laser.inital_direction, laser.direction));
            laser.direction = laser.direction.rotate(laser.change);
        }
    }

    pub fn reload_level(self, level: Level, name: String) -> Self {
        let mut engine = Self::new(self.channel, level);
        let mut stack = self.level_stack;
        stack.push(name);
        engine.level_stack = stack;
        engine
    }

    pub fn try_bind(&mut self, new_shape: &Rc<RefCell<dyn Collidable>>) {
        self.entities
            .iter_mut()
            .for_each(|shape| shape.try_bind(new_shape))
    }

    fn add_entity<S: Collidable + 'static>(
        &mut self,
        mut shape: S,
        entity_cfg: EntityCfg,
    ) -> Weak<RefCell<S>> {
        if entity_cfg.is_static {
            shape.collision_data_mut().mass = f64::INFINITY;
            shape.collision_data_mut().inertia = f64::INFINITY;
        }

        let shape = Rc::new(RefCell::new(shape));
        let shape_weak = Rc::downgrade(&shape);
        let shape_dyn: Rc<RefCell<dyn Collidable>> = shape;

        self.try_bind(&shape_dyn);
        self.entities.push(Entity::new(shape_dyn, entity_cfg));
        shape_weak
    }

    pub fn add_circle(&mut self, circle: Circle) {
        let weak_circle = self.add_entity(circle, EntityCfg::default());
        self.circles.push(weak_circle.into());
    }

    pub fn add_polygon(&mut self, polygon: Polygon) {
        let weak_polygon = self.add_entity(polygon, EntityCfg::default());
        self.polygons.push(weak_polygon.into());
    }

    pub fn erase_at(&mut self, point: Point) {
        if let Some(i) = self
            .entities
            .iter()
            .position(|shape| shape.shape.borrow().includes(point))
        {
            if self.entities[i].is_erasable {
                self.entities.remove(i);
            }
        }
    }

    pub fn add_hinge(&mut self, point: Point) {
        if let Some(i) = self
            .entities
            .iter()
            .position(|shape| shape.shape.borrow().includes(point) && shape.is_bindable)
        {
            self.entities[i].add_hinge(point);
        }
    }

    pub fn add_rigid(&mut self, point: Point) {
        if let Some(i) = self
            .entities
            .iter()
            .position(|shape| shape.shape.borrow().includes(point) && shape.is_bindable)
        {
            self.entities[i].add_rigid(point);
        }
    }

    pub fn jump(&mut self) {
        if self.jumps_count != 0 {
            let main_ball_mut = self.main_ball.upgrade().unwrap();
            main_ball_mut.borrow_mut().collision_data_mut().velocity +=
                Point(0.0, 1.0).rotate(-self.angle as f64);
            self.jumps_count -= 1;
        }
    }

    pub fn reset_level(&self) {
        let mut ball = self.entities[0].shape.borrow_mut();
        let data = ball.collision_data_mut();

        data.centroid = self.main_ball_starting_position;
        data.angular_velocity = 0.0;
        data.velocity = Vector::ZERO;
    }

    pub fn reset_jumps(&mut self) {
        self.jumps_count = 2;
    }
}

// #[cfg(test)]
// mod test {
//     use crate::levels;

//     use super::*;

//     fn init_engine() -> Engine {
//         Engine::new(
//             channel::bounded(1).0,
//             Level {
//                 initial_ball_position: Point(0.0, 0.5),
//                 polygons: vec![
//                     levels::Entity {
//                         is_bindable: false,
//                         is_static: true,
//                         shape: vec![
//                             Point(0.0, 0.0),
//                             Point(0.5, 0.0),
//                             Point(0.5, 0.5),
//                             Point(0.0, 0.5),
//                         ],
//                     },
//                     levels::Entity {
//                         is_bindable: false,
//                         is_static: true,
//                         shape: vec![
//                             Point(0.0, 1.0),
//                             Point(0.5, 1.0),
//                             Point(0.5, 1.5),
//                             Point(0.0, 1.5),
//                         ],
//                     },
//                 ],
//                 circles: vec![levels::Entity {
//                     is_bindable: false,
//                     is_static: true,
//                     shape: geometry::Circle {
//                         center: Point(0.0, 0.9),
//                         radius: 0.05,
//                     },
//                 }],
//                 flags_positions: vec![Point(-0.9, 0.0)],
//             },
//         )
//     }

//     #[test]
//     fn test_engine_creation() {
//         let engine = init_engine();

//         assert!(engine.circles.len() == 2);
//         assert!(engine.polygons.len() == 2);
//         assert!(engine.entities.len() == 4);
//         assert!(
//             engine.polygons[1]
//                 .shape
//                 .upgrade()
//                 .unwrap()
//                 .borrow_mut()
//                 .collision_data_mut()
//                 .mass
//                 == f64::INFINITY
//         );
//     }

//     #[test]
//     fn test_auto_bind() {
//         let mut engine = init_engine();

//         engine.add_polygon(make_shape! {
//             (-1.0, -1.0),
//             (-0.9, -1.0),
//             (-0.9, -0.9),
//             (-1.0, -0.9),
//         });

//         engine.add_rigid(Point(-0.91, -0.91));

//         assert!(engine.entities.last().unwrap().unbound.len() == 1);

//         engine.add_polygon(make_shape! {
//             (-0.92, -0.92),
//             (-0.85, -0.92),
//             (-0.85, -0.85),
//             (-0.92, -0.85),
//         });

//         let [.., first, second] = &engine.entities[..] else {
//             panic!("not enough enitites");
//         };

//         assert!(first.unbound.is_empty());
//         assert!(std::ptr::eq(
//             first.bindings[0].1.as_ptr() as *const c_void,
//             &*second.shape as *const _ as *const c_void
//         ));
//     }
// }
