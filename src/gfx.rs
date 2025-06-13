use wgpu::{self, util::DeviceExt};
use std::sync::Arc;
use winit::window::Window;
use std::time::Instant;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AngleUniform {
    angle: f32,
    _pad: [f32; 3],
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device:  wgpu::Device,
    queue:   wgpu::Queue,
    config:  wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    angle_buffer: wgpu::Buffer,
    angle_bind_group: wgpu::BindGroup,
    start_time: Instant,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Self {
        // creating instance
        let instance = wgpu::Instance::default();
        // get a surface
        let surface = instance.create_surface(window.clone()).unwrap();
        // get an adapter using adapteroptions
        let adapter = instance
        .request_adapter(
            &wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            }
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor::default()
        ).await.unwrap();
        // configure the surface
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Square vertices: 6 vertices for 2 triangles
        let vertices = [
            // Triangle 1: bottom-left, bottom-right, top-right
            -0.5_f32, -0.5, 1.0, 0.0, 0.0,  // bottom-left, red
             0.5, -0.5, 0.0, 1.0, 0.0,       // bottom-right, green
             0.5,  0.5, 0.0, 0.0, 1.0,       // top-right, blue
            
            // Triangle 2: bottom-left, top-right, top-left  
            -0.5, -0.5, 1.0, 0.0, 0.0,       // bottom-left, red (repeated)
             0.5,  0.5, 0.0, 0.0, 1.0,       // top-right, blue (repeated)
            -0.5,  0.5, 1.0, 1.0, 0.0,       // top-left, yellow
        ];

        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // init angle: 
        let angle_init = AngleUniform { angle: 0.0, _pad: [0.0; 3] };

        // Create angle buffer
        let angle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Angle UBO"),
            contents: bytemuck::bytes_of(&angle_init),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let angle_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Angle BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<AngleUniform>() as u64),
                },
                count: None,
            }]
        });

        let angle_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Angle BG"),
            layout: &angle_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: angle_buffer.as_entire_binding()
            }]
        });

        // Load WGSL shader from external file
        let shader_source = include_str!("shader.wgsl");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&angle_bgl],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // Position
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // Color
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self { surface, device, queue, config, render_pipeline, vertex_buffer, angle_buffer, angle_bind_group: angle_bg, start_time: Instant::now() }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("render encoder")
            }
        );

        // ---- update angle uniform ----
        let t = self.start_time.elapsed().as_secs_f32();
        let current = AngleUniform { angle: t, _pad: [0.0; 3] };
        self.queue.write_buffer(&self.angle_buffer, 0, bytemuck::bytes_of(&current));

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2, 
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.angle_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}