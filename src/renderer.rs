use crate::error::Error;
use bytemuck::{cast_slice, Pod, Zeroable};
use log::info;
use std::sync::Arc;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Adapter, BlendState, Buffer, BufferAddress, BufferUsages, ColorTargetState, ColorWrites,
    Device, Face, FragmentState, FrontFace, Instance, MultisampleState, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, Surface, SurfaceConfiguration,
    TextureView, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use winit::window::Window;

pub fn init(window: &Window) -> Result<Renderer, Error> {
    let renderer = pollster::block_on(Renderer::new(window));
    info!("renderer initialised");

    Ok(renderer)
}

pub struct Renderer {
    _instance: Instance,
    _adapter: Adapter,
    pub surface: Arc<Surface>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface_config: SurfaceConfiguration,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub output_texture: Option<TextureView>,

    quad: Quad,
}

impl Renderer {
    async fn new(window: &Window) -> Renderer {
        let size = window.inner_size();
        let width = size.width;
        let height = size.height;
        let scale_factor = window.scale_factor();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        // if let surface_
        let surface_format = surface.get_preferred_format(&adapter).unwrap();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &surface_config);

        let quad = Quad::init(&device, &surface_config);

        Self {
            _instance: instance,
            _adapter: adapter,
            surface: Arc::new(surface),
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface_config,
            width,
            height,
            scale_factor,
            output_texture: None,

            quad,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.scale_factor = scale_factor;
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn draw_quad(&mut self) {
        let (output, view) = if let Some(view) = self.output_texture.take() {
            (None, view)
        } else {
            let output = self
                .surface
                .get_current_texture()
                .expect("should have a surface");
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            (Some(output), view)
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what [[location(0)]] in the fragment shader targets
                    wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    },
                ],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.quad.render_pipeline);
            render_pass.set_vertex_buffer(0, self.quad.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.quad.index_buffer.slice(..),
                self.quad.index_buffer_format,
            );
            render_pass.draw_indexed(0..self.quad.num_indices, 0, 0..1);
        }

        let command_buffers = vec![encoder.finish()];
        self.queue.submit(command_buffers);

        if let Some(output) = output {
            output.present();
        } else {
            self.output_texture = Some(view);
        }
    }

    pub fn render_to_texture(&mut self, texture: Option<TextureView>) {
        self.output_texture = texture;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[rustfmt::skip]
const VERTICES: &[Vertex] = &[
    Vertex { position: [0.5, 0.5, 0.0], color: [0.9, 0.8, 0.2] }, // Top right.
    Vertex { position: [-0.5, 0.5, 0.0], color: [0.9, 0.8, 0.2] }, // Top left.
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.9, 0.8, 0.2] }, // Bottom left.
    Vertex { position: [0.5, -0.5, 0.0], color: [0.9, 0.8, 0.2] }, // Bottom right.
];

#[rustfmt::skip]
const INDICES: &[u16] = &[
    // Counter-clockwise because we specify `front_face: FrontFace::Ccw` in the render pipeline.
    0, 1, 2,
    0, 2, 3,
];

pub struct Quad {
    pub render_pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_indices: u32,
    pub index_buffer_format: wgpu::IndexFormat,
}

impl Quad {
    pub fn init(device: &Device, surface_config: &SurfaceConfiguration) -> Self {
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("../resources/shaders/quad.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[ColorTargetState {
                    format: surface_config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        });

        let num_indices = INDICES.len() as u32;

        let index_buffer_format = wgpu::IndexFormat::Uint16;

        Quad {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            index_buffer_format,
        }
    }
}
