pub mod turntable;

use clap::Parser;
use color_eyre::eyre::{bail, Result};
use glam::{vec3, Vec3};
use manifest_dir_macros::directory_relative_path;
use stardust_xr_fusion::{
    client::Client,
    core::values::ResourceID,
    drawable::Model,
    node::{MethodResult, NodeError, NodeType},
    root::{ClientState, FrameInfo, RootAspect, RootHandler},
    spatial::{SpatialAspect, SpatialRefAspect, Transform},
};
use std::{path::PathBuf, sync::Arc};
use tracing_subscriber::EnvFilter;
use turntable::{Turntable, TurntableSettings};

#[derive(Parser)]
pub struct Args {
    file_path: PathBuf,
}

struct Root {
    turntable: Turntable,
    _model: Model,
}
impl Root {
    async fn new(client: Arc<Client>, args: Args, radius: f32) -> Result<Self> {
        let model = Model::create(
            client.get_root(),
            Transform::from_translation([0.0; 3]),
            &ResourceID::new_direct(
                args.file_path
                    .canonicalize()
                    .map_err(|_| NodeError::InvalidPath)?,
            )?,
        )?;
        let model_bounds = model.get_relative_bounding_box(client.get_root()).await?;
        dbg!(&model_bounds);
        let max_model_dim = model_bounds
            .size
            .x
            .max(model_bounds.size.y.max(model_bounds.size.z));
        let mut scale = radius * 2.0 / max_model_dim;
        scale = scale.min(1.0);
        let turntable = Turntable::create(
            client.get_root(),
            Transform::identity(),
            TurntableSettings {
                line_count: 106,
                line_thickness: 0.002,
                height: 0.03,
                inner_radius: radius,
                scroll_multiplier: 10.0_f32.to_radians(),
            },
        )?;
        model.set_spatial_parent(turntable.content_parent())?;
        let mut position = vec3(0.0, model_bounds.size.y * scale / 2.0, 0.0);
        position -= Vec3::from(model_bounds.center) * scale * 0.5;
        model.set_local_transform(Transform::from_translation_scale(position, [scale; 3]))?;
        turntable.root().set_zoneable(true)?;
        Ok(Root {
            turntable,
            _model: model,
        })
    }
}
impl RootHandler for Root {
    fn frame(&mut self, info: FrameInfo) {
        self.turntable.update(info);
    }
    fn save_state(&mut self) -> MethodResult<ClientState> {
        ClientState::from_root(self.turntable.root())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();
    let args = Args::parse();
    let (client, event_loop) = Client::connect_with_async_loop().await?;

    let _wrapped_root = client
        .get_root()
        .alias()
        .wrap(Root::new(client.clone(), args, 0.1).await?)?;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => Ok(()),
        _ = event_loop => bail!("Server crashed"),
    }
}
