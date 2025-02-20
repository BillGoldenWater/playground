use std::sync::Arc;

use anyhow::{anyhow, Context};
use functional_utils::FunctionalUtils;
use wgpu::{
    Device, PresentMode, Surface, SurfaceConfiguration,
    TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

use self::renderer::Renderer;
use crate::wgpu_context::WgpuContext;

pub mod renderer;

#[derive(Debug)]
pub struct Viewport {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub renderer: Renderer,
}

impl Viewport {
    pub fn new(
        window: Arc<Window>,
        ctx: &WgpuContext,
        build_renderer: impl FnOnce(&WgpuContext, &Surface) -> Renderer,
    ) -> anyhow::Result<Self> {
        let surface = ctx
            .instance
            .create_surface(window.clone())
            .context("failed to create render surface")?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(
                &ctx.adapter,
                size.width.max(1),
                size.height.max(1),
            )
            .ok_or(anyhow!("failed to get default surface config"))?;
        config.present_mode = PresentMode::Immediate;

        surface.configure(&ctx.device, &config);

        let renderer = build_renderer(ctx, &surface);

        Self {
            window,
            surface,
            config,
            renderer,
        }
        .into_ok()
    }

    pub fn resize(&mut self, device: &Device, size: PhysicalSize<u32>) {
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);

        self.surface.configure(device, &self.config);
    }

    pub fn render(&self, ctx: &WgpuContext) -> anyhow::Result<()> {
        let frame = self
            .surface
            .get_current_texture()
            .context("failed to get next swapchain texture")?;
        let view =
            frame.texture.create_view(&TextureViewDescriptor::default());

        self.renderer.render(ctx, &view);
        frame.present();

        Ok(())
    }
}
