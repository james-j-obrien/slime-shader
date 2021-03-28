use std::{borrow::Cow, f32::consts::PI};

use wgpu::util::DeviceExt;

use crate::framework;
use crate::constants::*;

#[repr(C)]
#[derive(Copy, Clone)]
struct Params {
    num_agents: u32,
    width: u32,
    height: u32,
    speed: f32,
}

unsafe impl bytemuck::Zeroable for Params {}
unsafe impl bytemuck::Pod for Params {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct FrameData {
    frame_num: u32,
    delta : f32,
}

unsafe impl bytemuck::Zeroable for FrameData {}
unsafe impl bytemuck::Pod for FrameData {}


/// Example struct holds references to wgpu resources and frame persistent data
pub struct Shader {
    render_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,

    compute_bind_group: wgpu::BindGroup,
    agent_pipeline: wgpu::ComputePipeline,
    post_pipeline: wgpu::ComputePipeline,

    texture_buffer: wgpu::Buffer,
    texture: wgpu::Texture,

    frame_data_buffer: wgpu::Buffer,
    staging_belt: wgpu::util::StagingBelt,
    frame_num: usize,
}

impl Shader {
    fn setup_compute_bind(
        device: &wgpu::Device,
        texture: &wgpu::Buffer
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup, wgpu::Buffer) {
        let params = Params {
            num_agents: NUM_AGENTS,
            width: WIDTH,
            height: HEIGHT,
            speed: 100.0,
        };

        let param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Param buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsage::UNIFORM
        });

        let mut initial_agent_data = vec![0.0f32; (3 * NUM_AGENTS) as usize];
        for agent in initial_agent_data.chunks_mut(3) {
            agent[0] = WIDTH as f32 * rand::random::<f32>();
            agent[1] = HEIGHT as f32 * rand::random::<f32>();
            agent[2] = rand::random::<f32>() * PI * 2.0;
        }
        let agent_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Agent buffer"),
            contents: bytemuck::cast_slice(&initial_agent_data),
            usage: wgpu::BufferUsage::STORAGE
        });

        let frame_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame data buffer"),
            size: std::mem::size_of::<FrameData>() as _,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Params>() as _),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<FrameData>() as _)
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size:  wgpu::BufferSize::new((WIDTH * HEIGHT * 4 * 4).into()),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: None,
            });

        let compute_bind_group = device.create_bind_group( &wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: frame_data_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: texture.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: agent_buffer.as_entire_binding(),
                    }
                ],
                label: None,
            });

        (compute_bind_group_layout, compute_bind_group, frame_data_buffer)
    }

    fn setup_agent_compute(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        bind_group: &wgpu::BindGroupLayout,
    ) -> wgpu::ComputePipeline {
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Agents compute pipeline layout"),
                bind_group_layouts: &[bind_group],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Agents compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "update_agents",
        });

        compute_pipeline
    }

    fn setup_post_compute(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        bind_group: &wgpu::BindGroupLayout,
    ) -> wgpu::ComputePipeline {

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Post process compute pipeline layout"),
                bind_group_layouts: &[bind_group],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Post process compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "post_process",
        });

        compute_pipeline
    }

    fn setup_render(
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        texture: &wgpu::Texture
    ) -> (wgpu::BindGroup, wgpu::RenderPipeline) {
        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                }
            ]
        });
        // create render pipeline with empty bind group layout
        let texture_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render pipeline layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 1.0,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
        });

        let texture_view = texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render bind group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler)
                }
            ]
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&texture_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_texture",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_texture",
                targets: &[sc_desc.format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        (render_bind_group, render_pipeline)
    }
}

impl framework::Shader for Shader {
    /// constructs initial instance of Example struct
    fn init(
        sc_desc: &wgpu::SwapChainDescriptor,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {
        // load and compile the shader
        let mut flags = wgpu::ShaderFlags::VALIDATION;
        match adapter.get_info().backend {
            wgpu::Backend::Vulkan | wgpu::Backend::Metal | wgpu::Backend::Gl => {
                flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION;
            }
            _ => {} //TODO
        }
        let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("compute.wgsl"))),
            flags,
        });
        let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("draw.wgsl"))),
            flags,
        });

        let texture_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Texture buffer"),
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_SRC,
            size: WIDTH as u64 * HEIGHT as u64 * 4 * 4,
            mapped_at_creation: false,
        });

        let texture_format = wgpu::TextureFormat::Rgba32Float;
        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("Texture descriptor"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsage::SAMPLED 
                | wgpu::TextureUsage::RENDER_ATTACHMENT 
                | wgpu::TextureUsage::COPY_DST
        };
        let texture = device
            .create_texture(&texture_descriptor);
        let (compute_bind_group_layout, compute_bind_group, frame_data_buffer) =
            Self::setup_compute_bind(device, &texture_buffer);

        let agent_pipeline = 
            Self::setup_agent_compute(device, &compute_shader, &compute_bind_group_layout);

        let post_pipeline =
            Self::setup_post_compute(device, &compute_shader, &compute_bind_group_layout);

        let (render_bind_group, render_pipeline) =
            Self::setup_render(sc_desc, device, &draw_shader, &texture);

        Shader {
            texture_buffer,
            texture,

            compute_bind_group,
            agent_pipeline,
            post_pipeline,

            render_bind_group,
            render_pipeline,
            
            frame_data_buffer,

            staging_belt: wgpu::util::StagingBelt::new(16),
            frame_num: 0,
        }
    }

    /// update is called for any WindowEvent not handled by the framework
    fn update(&mut self, _event: winit::event::WindowEvent) {
        //empty
    }

    /// resize is called on WindowEvent::Resized events
    fn resize(
        &mut self,
        _sc_desc: &wgpu::SwapChainDescriptor,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        //empty
    }

    /// render is called each frame, dispatching compute groups proportional
    ///   a TriangleList draw call for all NUM_PARTICLES at 3 vertices each
    fn render(
        &mut self,
        frame: &wgpu::SwapChainTexture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        spawner: &framework::Spawner,
        delta: std::time::Duration
    ) {
        // create render pass descriptor and its color attachments
        let color_attachments = [wgpu::RenderPassColorAttachment {
            view: &frame.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: true,
            },
        }];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
        };

        let frame_data = FrameData {
            frame_num: self.frame_num as _,
            delta: delta.as_secs_f32()
        };

        // println!("{:?}", frame_data);

        // get command encoder
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.staging_belt
            .write_buffer(
                &mut command_encoder,
                &self.frame_data_buffer,
                0,
                wgpu::BufferSize::new(std::mem::size_of::<FrameData>() as _).unwrap(),
                device,
            )
            .copy_from_slice(bytemuck::bytes_of(&frame_data));

        self.staging_belt.finish();
        command_encoder.push_debug_group("compute agent movement");
        {
            // compute pass
            let mut cpass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.agent_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.dispatch(NUM_AGENTS, 1, 1);
        }
        command_encoder.pop_debug_group();
        command_encoder.push_debug_group("compute post processing");
        {
            let mut cpass = 
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.post_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.dispatch(WIDTH, HEIGHT, 1);
        }
        command_encoder.copy_buffer_to_texture(
            wgpu::ImageCopyBuffer {
                buffer: &self.texture_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(std::num::NonZeroU32::new(WIDTH * 16).unwrap()), //Texel size
                    rows_per_image: Some(std::num::NonZeroU32::new(HEIGHT).unwrap()),
                },
            }, 
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::default()
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            });

        command_encoder.push_debug_group("render texture");
        {
            // render pass
            let mut rpass = command_encoder.begin_render_pass(&render_pass_descriptor);

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.render_bind_group, &[]);
            rpass.draw(0..6, 0..1);
        }
        command_encoder.pop_debug_group();

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));

        let belt_future = self.staging_belt.recall();
        spawner.spawn_local(belt_future);

    }
}
