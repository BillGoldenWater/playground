use std::{
    collections::VecDeque,
    mem::size_of,
    sync::{Arc, Mutex},
    time::Instant,
};

use bytemuck::cast_slice;
use tracing::info;
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferAddress, BufferBindingType, BufferDescriptor,
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Face, LoadOp, Operations,
    PipelineLayoutDescriptor, PushConstantRange, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, ShaderStages, StoreOp, Surface, TextureView, VertexBufferLayout,
    VertexStepMode,
};
use wgpu_bitonic_sort::BitonicSorter;

use self::{command::Command, param::Param, point::Point};
use crate::wgpu_context::WgpuContext;

pub mod command;
pub mod param;
pub mod point;

#[derive(Debug)]
pub struct Renderer {
    pub last_update: Instant,

    pub input_state: Arc<Mutex<Param>>,
    pub command_queue: Arc<Mutex<VecDeque<Command>>>,

    pub points: Vec<Point>,
    pub points_buffer: Buffer,
    pub points_out_buffer: Buffer,

    pub points_hash_data_buffer: Buffer,
    pub points_hash_index_buffer: Buffer,

    pub compute_bind_group: BindGroup,

    pub calc_hash_data_pipeline: ComputePipeline,
    pub hash_data_sorter: BitonicSorter,
    pub calc_hash_index_pipeline: ComputePipeline,
    pub compute_pipeline: ComputePipeline,
    pub render_pipeline: RenderPipeline,
}

impl Renderer {
    pub fn new(
        ctx: &WgpuContext,
        surface: &Surface,
        input_state: Arc<Mutex<Param>>,
        command_queue: Arc<Mutex<VecDeque<Command>>>,
    ) -> Self {
        let WgpuContext {
            adapter, device, ..
        } = &ctx;

        // data
        let points = Point::gen();

        let points_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("points_buffer"),
            contents: cast_slice(&points),
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let points_out_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("points_out_buffer"),
            contents: cast_slice(&points),
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX | BufferUsages::COPY_SRC,
        });

        let points_hash_data_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("points_hash_data_buffer"),
            size: (4 + 4) * points.len() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let points_hash_index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("points_hash_index_buffer"),
            size: 4 * points.len() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("compute_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // stats out
        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("compute_bind_group"),
            layout: &compute_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: points_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: points_out_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: points_hash_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: points_hash_index_buffer.as_entire_binding(),
                },
            ],
        });

        // pipeline
        let shader = device.create_shader_module(include_wgsl!("../../../shader.wgsl"));

        // compute pipeline
        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("compute layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..size_of::<Param>() as u32,
            }],
        });

        let calc_hash_data_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("calc hash data pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "calc_hash_data",
            compilation_options: Default::default(),
        });

        let calc_hash_index_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("calc hash index pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "calc_hash_index",
            compilation_options: Default::default(),
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "cs_main",
            compilation_options: Default::default(),
        });

        // render pipeline
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let instance_buffer_layout = VertexBufferLayout {
            array_stride: size_of::<Point>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x2],
        };

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[instance_buffer_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: swapchain_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::COLOR,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let hash_data_sorter = BitonicSorter::new(
            &device,
            &points_hash_data_buffer,
            "index: u32, hash: u32,",
            "a.hash > b.hash",
        );

        Self {
            last_update: Instant::now(),

            input_state,
            command_queue,

            points,
            points_buffer,
            points_out_buffer,

            points_hash_data_buffer,
            points_hash_index_buffer,

            compute_bind_group,

            calc_hash_data_pipeline,
            hash_data_sorter,
            calc_hash_index_pipeline,
            compute_pipeline,
            render_pipeline,
        }
    }

    pub fn update(&mut self, ctx: &WgpuContext) {
        // time delta
        // let time_delta = self.last_update.elapsed().as_secs_f32();
        self.last_update = Instant::now();

        // command
        let mut cmd_queue = self.command_queue.lock().unwrap();

        while let Some(command) = cmd_queue.pop_front() {
            info!("on command: {command:?}");
            match command {
                Command::Reset => {
                    ctx.queue
                        .write_buffer(&self.points_buffer, 0, cast_slice(&self.points));
                }
            }
        }

        // input state & param
        let state = self.input_state.lock().unwrap();
        let param = [Param {
            time_delta: 1f32 / 1000.0,
            ..*state
        }];
        let param_slice = cast_slice::<_, u8>(&param);

        // dimensions
        let size = self.points.len() as f64;
        let x = (size as u32).min(65535);
        let y = ((size / 65535.0).ceil() as u32).min(65535);
        let z = (size / 65535.0 / 65535.0).ceil() as u32;

        // hash data
        {
            let mut encoder = ctx
                .device
                .create_command_encoder(&CommandEncoderDescriptor { label: None });

            {
                let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("hash data compute pass"),
                    timestamp_writes: None,
                });

                pass.set_pipeline(&self.calc_hash_data_pipeline);
                pass.set_bind_group(0, &self.compute_bind_group, &[]);
                pass.dispatch_workgroups(x, y, z);
            }

            ctx.queue.submit(Some(encoder.finish()));
        }

        self.hash_data_sorter
            .sort(&ctx.device, &ctx.queue, self.points.len() as u32);

        // hash index & update points
        {
            let mut encoder = ctx
                .device
                .create_command_encoder(&CommandEncoderDescriptor { label: None });

            {
                let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("hash index & update points compute pass"),
                    timestamp_writes: None,
                });

                pass.set_pipeline(&self.calc_hash_index_pipeline);
                pass.set_bind_group(0, &self.compute_bind_group, &[]);
                pass.dispatch_workgroups(x, y, z);

                pass.set_pipeline(&self.compute_pipeline);
                pass.set_push_constants(0, param_slice);
                pass.set_bind_group(0, &self.compute_bind_group, &[]);
                pass.dispatch_workgroups(x, y, z);
            }

            encoder.copy_buffer_to_buffer(
                &self.points_out_buffer,
                0,
                &self.points_buffer,
                0,
                (size_of::<Point>() * self.points.len()) as BufferAddress,
            );

            ctx.queue.submit(Some(encoder.finish()));
        }

        // ctx.device.poll(wgpu::MaintainBase::Wait).panic_on_timeout();
    }

    pub fn render(&self, ctx: &WgpuContext, view: &TextureView) {
        let mut encoder = ctx
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_vertex_buffer(0, self.points_out_buffer.slice(..));

            rpass.draw(0..6, 0..self.points.len() as u32);
        }

        ctx.queue.submit(Some(encoder.finish()));
    }
}
