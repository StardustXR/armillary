use clap::Parser;
use serde::{Deserialize, Serialize};
use stardust_xr_asteroids::{
    client::ClientState,
    elements::{BoundsTransformer, FileWatcher, GrabRing, Model, Text, Turntable},
    Context, CustomElement as _, Element, Migrate, Reify, Tasker, Transformable,
};
use stardust_xr_fusion::{drawable::XAlign, spatial::Transform, types::Vec3F};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
pub struct Args {
    file_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pos: Vec3F,
    model_path: PathBuf,
    turntable_angle: f32,
    radius: f32,

    #[serde(skip)]
    model_loaded: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            pos: [0.0; 3].into(),
            model_path: PathBuf::new(),
            turntable_angle: 0.0,
            radius: 0.1,
            model_loaded: false,
        }
    }
}
impl Migrate for State {
    type Old = Self;
}
impl ClientState for State {
    const APP_ID: &'static str = "org.stardustxr.armillary";
    fn initial_state_update(&mut self) {
        let args = Args::parse();
        self.model_path = args.file_path.canonicalize().unwrap();
    }
}
impl Reify for State {
    fn reify(&self, _context: &Context, _tasks: impl Tasker<Self>) -> impl Element<Self> {
        let mut model = None;
        let mut model_error = None;
        match Model::direct(&self.model_path) {
            Ok(model_elem) => model = Some(model_elem.build()),
            Err(e) => {
                model_error = Some(
                    Text::new(format!("Model Error:\n{e}"))
                        .align_x(XAlign::Center)
                        .character_height(0.025)
                        .pos([0.0, 0.075, 0.0])
                        .build(),
                )
            }
        };
        model.take_if(|_| !self.model_loaded);
        GrabRing::new(self.pos, |state: &mut State, pos| {
            state.pos = pos;
        })
        .radius(self.radius + 0.04)
        .build()
        .child(
            Turntable::new(self.turntable_angle, |state: &mut State, angle| {
                state.turntable_angle = angle;
            })
            .pos([0.0, 0.035, 0.0])
            .inner_radius(self.radius)
            .build()
            .child(
                BoundsTransformer::new({
                    let radius = self.radius;
                    move |bounds| {
                        let height_offset = (bounds.extents.y / 2.0) - bounds.center.y;

                        dbg!(bounds.extents);

                        let max_size = bounds.extents.x.max(bounds.extents.z);
                        let scale = radius * 2.0 / max_size;

                        Transform::from_translation_scale([0.0, height_offset, 0.0], [scale; 3])
                    }
                })
                .build()
                .maybe_child(model)
                .maybe_child(model_error),
            )
            .child(
                FileWatcher::new(self.model_path.clone(), |state: &mut State| {
                    println!("file is modified");
                    state.model_loaded = false;
                })
                .build(),
            ),
        )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();
    // let args = Args::parse();
    stardust_xr_asteroids::client::run::<State>(&[])
        .await
        .unwrap();
}
