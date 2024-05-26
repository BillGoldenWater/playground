use anyhow::{anyhow, Context};
use functional_utils::FunctionalUtils;
use tracing::info;
use wgpu::{Adapter, Device, Features, Instance, PowerPreference, Queue, RequestAdapterOptions};

#[derive(Debug)]
pub struct WgpuContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl WgpuContext {
    pub async fn new() -> anyhow::Result<Self> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or(anyhow!("no adapter available"))?;

        info!("Adapter selected: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits: adapter.limits(),
                    required_features: adapter.features() | Features::PUSH_CONSTANTS,
                    ..Default::default()
                },
                None,
            )
            .await
            .context("falied to request device")?;

        Self {
            instance,
            adapter,
            device,
            queue,
        }
        .into_ok()
    }
}
