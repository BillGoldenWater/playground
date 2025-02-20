use std::iter::successors;

use bytemuck::cast_slice;
use param::Param;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    CommandBuffer, CommandEncoderDescriptor, ComputePassDescriptor,
    ComputePipeline, ComputePipelineDescriptor, Device,
    PipelineCompilationOptions, PipelineLayoutDescriptor,
    PushConstantRange, Queue, ShaderModuleDescriptor, ShaderSource,
    ShaderStages,
};

pub mod param;

#[derive(Debug)]
pub struct BitonicSorter {
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,

    pipeline: ComputePipeline,
}

impl BitonicSorter {
    pub fn new(
        device: &Device,
        target_buffer: &Buffer,
        data_member_def: &str,
        data_cmp_expr: &str,
    ) -> Self {
        let shader_src = include_str!("./bitonic_sort.wgsl");

        let shader_src = shader_src
            .replace("value: u32,", data_member_def)
            .replace("a.value > b.value", data_cmp_expr);

        let shader = device.create_shader_module({
            ShaderModuleDescriptor {
                label: Some("./bitonic_sort.wgsl"),
                source: ShaderSource::Wgsl(shader_src.into()),
            }
        });

        let bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bitonic sort bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let bind_group = Self::create_bind_group(
            device,
            target_buffer,
            &bind_group_layout,
        );

        let pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("bitonic sort compute pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..(4 * 5),
                }],
            });

        let pipeline =
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("bitonic sort compute pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("bitonic_sort_op"),
                compilation_options: PipelineCompilationOptions::default(
                ),
                cache: None,
            });

        Self {
            bind_group_layout,
            bind_group,
            pipeline,
        }
    }

    fn create_bind_group(
        device: &Device,
        target_buffer: &Buffer,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("bitonic sort bind group"),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: target_buffer.as_entire_binding(),
            }],
        })
    }

    pub fn change_buffer(
        &mut self,
        device: &Device,
        target_buffer: &Buffer,
    ) {
        self.bind_group = Self::create_bind_group(
            device,
            target_buffer,
            &self.bind_group_layout,
        )
    }

    pub fn sort(&self, device: &Device, queue: &Queue, data_len: u32) {
        queue.submit([self.sort_command_buffer(device, data_len)]);
    }

    pub fn sort_command_buffer(
        &self,
        device: &Device,
        data_len: u32,
    ) -> CommandBuffer {
        let max_dim_size =
            device.limits().max_compute_workgroups_per_dimension;

        let mut encoder =
            device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("bitonic sort command encoder"),
            });

        if data_len != 0 {
            let mut pass =
                encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("bitonic sort compute pass"),
                    timestamp_writes: None,
                });

            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_pipeline(&self.pipeline);

            let len = data_len.next_power_of_two();
            let size = ((len / 2) as f64).cbrt().ceil() as u32;
            // incorrect, but works when not exceeding limit
            let size = size.min(max_dim_size);

            for stage in successors(Some(2_u32), |it| it.checked_mul(2))
                .take_while(|&it| it <= len)
            {
                for step in
                    successors(Some(stage / 2), |it| it.checked_div(2))
                        .take_while(|&it| it > 0)
                {
                    let step_log2 = step.trailing_zeros();
                    let step_mod_mask = ((step - 1) | step) >> 1;

                    pass.set_push_constants(
                        0,
                        cast_slice(&[Param {
                            dimension_size: size,

                            stage,
                            step,
                            step_log2,
                            step_mod_mask,
                        }]),
                    );

                    pass.dispatch_workgroups(size, size, size);
                }
            }
        }

        encoder.finish()
    }
}

#[cfg(test)]
mod tests {
    use rand::{Rng as _, SeedableRng};
    use wgpu::{
        util::DeviceExt as _, BufferAddress, BufferUsages, Features,
        MapMode, RequestAdapterOptions,
    };

    use super::*;

    async fn init_ctx() -> (Device, Queue) {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .expect("no adapter available");

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits: adapter.limits(),
                    required_features: adapter.features()
                        | Features::PUSH_CONSTANTS,
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("falied to request device")
    }

    async fn sort(mut data: Vec<u32>) {
        // prepare
        let (device, queue) = init_ctx().await;

        let data_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("bitonic sort test data buffer"),
                contents: cast_slice(&data),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            },
        );

        let data_map_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("bitonic sort test data mapping buffer"),
                contents: cast_slice(&data),
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            },
        );

        // GPU sort
        let sorter = BitonicSorter::new(
            &device,
            &data_buffer,
            "value: u32",
            "a.value > b.value",
        );
        sorter.sort(&device, &queue, data.len() as u32);

        // copy buffer
        let mut encoder =
            device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("command encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &data_buffer,
            0,
            &data_map_buffer,
            0,
            (data.len() * 4) as BufferAddress,
        );
        queue.submit([encoder.finish()]);

        // map GPU sorted
        let slice = data_map_buffer.slice(..);
        slice.map_async(MapMode::Read, |_| {});

        device.poll(wgpu::MaintainBase::Wait).panic_on_timeout();

        let view = slice.get_mapped_range();
        let gpu_sorted: &[u32] = cast_slice(&view);

        // std sort
        data.sort();
        let std_sorted = data;

        // assert_eq would cause huge output when failed
        assert!(gpu_sorted == std_sorted);
    }

    #[tokio::test]
    async fn test_sort_rand() {
        run_sort_rand(1, 16384).await;
        run_sort_rand(1, 16385).await;
        run_sort_rand(1, 17408).await;
        run_sort_rand(1, 1_000_000).await;
    }

    async fn run_sort_rand(seed: u64, n: usize) {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);

        let data = std::iter::repeat(0)
            .take(n)
            .map(|_| rng.gen_range(0..u32::MAX))
            .collect();

        sort(data).await;
    }

    #[tokio::test]
    async fn test_sort_seq() {
        sort((0..16384).collect()).await;
        sort((0..16385).collect()).await;
        sort((0..17408).collect()).await;
        sort((0..1_000_000).collect()).await;
    }

    #[tokio::test]
    async fn test_sort_seq_rev() {
        sort((0..16384).rev().collect()).await;
        sort((0..16385).rev().collect()).await;
        sort((0..17408).rev().collect()).await;
        sort((0..1_000_000).rev().collect()).await;
    }
}
