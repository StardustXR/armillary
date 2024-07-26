use glam::{Quat, Vec3};
use map_range::MapRange;
use stardust_xr_fusion::{
    drawable::{Line, LinePoint, Lines, LinesAspect},
    fields::{CylinderShape, Field, Shape},
    input::{InputData, InputDataType, InputHandler},
    node::NodeError,
    root::FrameInfo,
    spatial::{Spatial, SpatialAspect, SpatialRefAspect, Transform},
    values::color::rgba_linear,
};
use stardust_xr_molecules::input_action::{InputQueue, InputQueueable, SimpleAction, SingleAction};
use std::f32::{
    consts::{FRAC_PI_2, TAU},
    INFINITY,
};

#[derive(Debug, Clone, Copy)]
pub struct TurntableSettings {
    pub line_count: u32,
    pub line_thickness: f32,
    pub height: f32,
    pub inner_radius: f32,
    pub scroll_multiplier: f32,
}
impl TurntableSettings {
    fn grip_lines(&self) -> Vec<Line> {
        (0..self.line_count)
            .into_iter()
            .map(|c| (c as f32) / (self.line_count as f32) * TAU) // get angle from count
            .map(|a| a.sin_cos()) // get x+y from angle (unit circle)
            .map(|(x, y)| {
                let outer_radius = self.inner_radius + self.height;
                Line {
                    points: vec![
                        LinePoint {
                            point: [x * self.inner_radius, 0.0, y * self.inner_radius].into(),
                            thickness: self.line_thickness,
                            color: rgba_linear!(1.0, 1.0, 1.0, 1.0),
                        },
                        LinePoint {
                            point: [x * outer_radius, -self.height, y * outer_radius].into(),
                            thickness: self.line_thickness,
                            color: rgba_linear!(1.0, 1.0, 1.0, 1.0),
                        },
                    ],
                    cyclic: false,
                }
            })
            .collect()
    }
}

fn interact_point(input: &InputData) -> Option<Vec3> {
    match &input.input {
        InputDataType::Hand(h) => {
            Some(Vec3::from(h.thumb.tip.position).lerp(Vec3::from(h.index.tip.position), 0.5))
        }
        InputDataType::Tip(t) => Some(t.origin.into()),
        _ => None,
    }
}
fn interact_points(input: &InputData) -> Vec<Vec3> {
    match &input.input {
        InputDataType::Hand(h) => {
            vec![
                h.thumb.tip.position.into(),
                h.index.tip.position.into(),
                h.ring.tip.position.into(),
                h.middle.tip.position.into(),
                h.little.tip.position.into(),
            ]
        }
        InputDataType::Tip(t) => vec![t.origin.into()],
        _ => vec![],
    }
}
fn interact_proximity(input: &InputQueue, point: Vec3) -> f32 {
    input
        .input()
        .keys()
        .flat_map(|i| match &i.input {
            InputDataType::Hand(h) => {
                vec![
                    h.thumb.tip.position,
                    h.index.tip.position,
                    h.ring.tip.position,
                    h.middle.tip.position,
                    h.little.tip.position,
                ]
            }
            InputDataType::Tip(t) => vec![t.origin],
            _ => vec![],
        })
        .map(|p| Vec3::from(p).distance(point))
        .reduce(|a, b| a.min(b))
        .unwrap_or(INFINITY)
}
fn interact_angle(input: &InputData) -> Option<f32> {
    let p = interact_point(input)?;
    Some(p.z.atan2(p.x))
}

pub struct Turntable {
    root: Spatial,
    content_parent: Spatial,
    settings: TurntableSettings,
    grip_lines: Vec<Line>,
    grip: Lines,
    _field: Field,

    input: InputQueue,
    pointer_hover_action: SimpleAction,
    touch_action: SingleAction,
    angular_momentum: f32,
    prev_angle: Option<f32>,
    rotation: f32,
}
impl Turntable {
    pub fn create(
        parent: &impl SpatialRefAspect,
        transform: Transform,
        settings: TurntableSettings,
    ) -> Result<Self, NodeError> {
        let root = Spatial::create(parent, transform, false)?;
        let content_parent = Spatial::create(&root, Transform::none(), false)?;
        let field = Field::create(
            &root,
            Transform::from_translation_rotation(
                [0.0, -settings.height * 0.5, 0.0],
                Quat::from_rotation_x(FRAC_PI_2),
            ),
            Shape::Cylinder(CylinderShape {
                length: settings.height,
                radius: settings.inner_radius + settings.height,
            }),
        )?;
        let input = InputHandler::create(&root, Transform::none(), &field)?.queue()?;

        let grip_lines: Vec<Line> = settings.grip_lines();
        let grip = Lines::create(&content_parent, Transform::none(), &grip_lines)?;

        Ok(Self {
            root,
            content_parent,
            settings,
            grip_lines,
            grip,
            _field: field,
            input,
            pointer_hover_action: Default::default(),
            touch_action: Default::default(),
            prev_angle: None,
            rotation: 0.0,
            angular_momentum: 0.0,
        })
    }

    pub fn root(&self) -> &Spatial {
        &self.root
    }
    pub fn content_parent(&self) -> &Spatial {
        &self.content_parent
    }

    #[inline]
    fn scroll(&self) -> f32 {
        self.pointer_hover_action
            .currently_acting()
            .iter()
            .map(|i| {
                i.datamap.with_data(|d| {
                    let scroll = d.idx("scroll_continuous").as_vector();
                    (scroll.idx(0).as_f32(), scroll.idx(1).as_f32())
                })
            })
            .map(|(scroll_x, scroll_y)| scroll_x + scroll_y)
            .reduce(|a, b| a + b)
            .unwrap_or_default()
    }
    pub fn rotate(&mut self, angle: f32) {
        self.rotation += angle;
        let _ = self
            .content_parent
            .set_local_transform(Transform::from_rotation(Quat::from_rotation_y(
                self.rotation,
            )));
    }

    pub fn update(&mut self, info: FrameInfo) {
        self.angular_momentum *= 0.98;

        self.pointer_hover_action
            .update(&self.input, &|input| match &input.input {
                InputDataType::Pointer(_) => input.distance < 0.0,
                _ => false,
            });
        self.touch_action.update(
            false,
            &self.input,
            |_| true,
            |input| {
                let slope_condition = interact_points(input).into_iter().any(|p| {
                    let h = p.y + self.settings.height;
                    let r = p.x.hypot(p.z) - self.settings.inner_radius;
                    h < r
                });
                let distance_condition = input.distance < 0.0;
                slope_condition && distance_condition
            },
        );

        self.rotate(-self.scroll() * self.settings.scroll_multiplier);

        // if touching
        if let Some(angle) = self
            .touch_action
            .actor()
            .cloned()
            .as_deref()
            .and_then(interact_angle)
        {
            if let Some(prev_angle) = self.prev_angle {
                let delta = prev_angle - angle;
                self.angular_momentum = delta * info.delta;
                self.rotate(delta);
            }
            self.prev_angle.replace(angle);
        }
        if self.touch_action.actor_stopped() {
            self.prev_angle.take();
        }
        if !self.touch_action.actor_acting() {
            self.rotate(self.angular_momentum / info.delta);
        }

        // update grip color
        for line in &mut self.grip_lines {
            for point in &mut line.points {
                let lerp = interact_proximity(
                    &self.input,
                    Quat::from_rotation_y(self.rotation) * Vec3::from(point.point),
                )
                .map_range(0.05..0.0, 1.0..0.0)
                .clamp(0.0, 1.0);
                point.color = rgba_linear!(lerp, lerp, lerp, 1.0);
            }
        }
        self.grip.set_lines(&self.grip_lines).unwrap();
    }
}
