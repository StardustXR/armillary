pub mod bounds;
pub mod file_watcher;
pub mod grab_ring;
pub mod turntable;

use asteroids::{
    client::ClientState,
    custom::{ElementTrait, Transformable},
    elements::{Model, Text},
    util::Migrate,
};
use bounds::Bounds;
use clap::Parser;
use file_watcher::FileWatcher;
use grab_ring::GrabRing;
use mint::Vector3;
use serde::{Deserialize, Serialize};
use stardust_xr_fusion::drawable::XAlign;
use std::{path::PathBuf, sync::OnceLock};
use tracing_subscriber::EnvFilter;
use turntable::Turntable;
use uuid::Uuid;

#[derive(Parser)]
pub struct Args {
    file_path: PathBuf,
}

#[derive(Debug)]
pub struct ModelInfo {
    uuid: Uuid,
    height_offset: f32,
    scale: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pos: Vector3<f32>,
    model_path: PathBuf,
    turntable_angle: f32,
    radius: f32,

    #[serde(skip)]
    model_info: OnceLock<ModelInfo>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            pos: [0.0; 3].into(),
            model_path: PathBuf::new(),
            turntable_angle: 0.0,
            radius: 0.1,
            model_info: OnceLock::new(),
        }
    }
}
impl Migrate for State {
    type Old = Self;
}
impl ClientState for State {
    const QUALIFIER: &'static str = "org";
    const ORGANIZATION: &'static str = "stardustxr";
    const NAME: &'static str = "armillary";

    fn initial_state_update(&mut self) {
        let args = Args::parse();
        self.model_path = args.file_path.canonicalize().unwrap();
    }
    fn reify(&self) -> asteroids::Element<Self> {
        let model_info = self.model_info.get_or_init(|| {
            let uuid = Uuid::new_v4();
            println!("creating new model info with uuid {uuid}");
            ModelInfo {
                uuid,
                height_offset: 0.0,
                scale: 0.0,
            }
        });

        let model = match Model::direct(&self.model_path) {
            Ok(model) => model
                .pos([0.0, model_info.height_offset, 0.0])
                .build()
                .identify(&model_info.uuid),
            Err(e) => Text::default()
                .text(format!("Model Error:\n{}", e))
                .text_align_x(XAlign::Center)
                .character_height(0.025)
                .pos([0.0, 0.075, 0.0])
                .build(),
        };

        let bounds = Bounds::new(|state: &mut State, bounds| {
            let Some(model_info) = state.model_info.get_mut() else {
                return;
            };

            model_info.height_offset = bounds.size.y / 2.0;

            let min_size = bounds.size.x.min(bounds.size.z);
            model_info.scale = state.radius * 2.0 / min_size;
        })
        .scl([model_info.scale; 3])
        .with_children([model]);

        let file_watcher = FileWatcher::new(self.model_path.clone(), |state: &mut State| {
            println!("file is modified");
            state.model_info.take();
        })
        .build();
        let turntable = Turntable::new(self.turntable_angle, |state: &mut State, angle| {
            state.turntable_angle = angle;
        })
        .pos([0.0, 0.035, 0.0])
        .inner_radius(self.radius)
        .with_children([bounds, file_watcher]);

        GrabRing::new(self.pos, |state: &mut State, pos| {
            state.pos = pos;
        })
        .radius(self.radius + 0.04)
        .with_children([turntable])
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();
    // let args = Args::parse();
    asteroids::client::run::<State>(&[]).await
}
