use crate::error::Error;
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use log::info;
use std::cell::RefCell;
use std::sync::Arc;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Adapter, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, Device, Face, FragmentState,
    FrontFace, Instance, MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, Queue, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, Surface, SurfaceConfiguration, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
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
    pub output_texture: Option<Arc<RefCell<TextureView>>>,

    rect_pipeline: RectPipeline,
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

        let surface_format = surface.get_preferred_format(&adapter).unwrap();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &surface_config);

        let rect_pipeline = RectPipeline::init(&device, &surface_config);

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

            rect_pipeline,
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

    pub fn draw_rect(&mut self, rect: &Rect) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let vertices = [
            Vertex::new([1.0, 1.0], rect.color),
            Vertex::new([0.0, 1.0], rect.color),
            Vertex::new([0.0, 0.0], rect.color),
            Vertex::new([1.0, 0.0], rect.color),
        ];

        #[rustfmt::skip]
        let indices: [u16; 6] = [
            0, 1, 2,
            0, 2, 3
        ];

        let vertex_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: BufferUsages::COPY_SRC,
        });

        let index_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: cast_slice(&indices),
            usage: BufferUsages::COPY_SRC,
        });

        let model_uniform_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Model Uniform Buffer"),
            contents: cast_slice(rect.scale_rotation_translation().as_ref()),
            usage: BufferUsages::COPY_SRC,
        });

        let view_ = glam::Mat4::look_at_lh(
            glam::Vec3::new(-200.0, -200.0, -1.0),
            glam::Vec3::new(-200.0, -200.0, 0.0),
            glam::Vec3::Y,
        );
        // TODO: Set where the origin is - might want center of screen, not bottom left.
        // TODO: Set Pixels-Per-Unit and scale things accordingly.
        let projection_ =
            glam::Mat4::orthographic_lh(0.0, self.width as f32, 0.0, self.height as f32, -1.0, 1.0);
        let vp = ViewProjectionUniform {
            view: view_.to_cols_array_2d(),
            // view: Mat4::IDENTITY.to_cols_array_2d(),
            projection: projection_.to_cols_array_2d(),
        };

        let view_projection_uniform_buffer =
            self.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("View Projection Uniform Buffer"),
                contents: cast_slice(&[vp]),
                usage: BufferUsages::COPY_SRC,
            });

        encoder.copy_buffer_to_buffer(
            &vertex_buffer,
            0,
            &self.rect_pipeline.vertex_buffer,
            0,
            (std::mem::size_of::<Vertex>() * vertices.len()) as BufferAddress,
        );

        encoder.copy_buffer_to_buffer(
            &index_buffer,
            0,
            &self.rect_pipeline.index_buffer,
            0,
            (std::mem::size_of::<u16>() * indices.len()) as BufferAddress,
        );

        encoder.copy_buffer_to_buffer(
            &model_uniform_buffer,
            0,
            &self.rect_pipeline.model_uniform_buffer,
            0,
            std::mem::size_of::<Mat4>() as BufferAddress,
        );

        encoder.copy_buffer_to_buffer(
            &view_projection_uniform_buffer,
            0,
            &self.rect_pipeline.view_projection_uniform_buffer,
            0,
            std::mem::size_of::<ViewProjectionUniform>() as BufferAddress,
        );

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
            (Some(output), Arc::new(RefCell::new(view)))
        };

        {
            let view = &view.as_ref().borrow();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what [[location(0)]] in the fragment shader targets
                    wgpu::RenderPassColorAttachment {
                        view,
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

            render_pass.set_pipeline(&self.rect_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &self.rect_pipeline.model_uniform_buffer_bind_group, &[]);
            render_pass.set_bind_group(
                1,
                &self.rect_pipeline.view_projection_uniform_buffer_bind_group,
                &[],
            );
            render_pass.set_vertex_buffer(0, self.rect_pipeline.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.rect_pipeline.index_buffer.slice(..),
                self.rect_pipeline.index_buffer_format,
            );
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        let command_buffers = vec![encoder.finish()];
        self.queue.submit(command_buffers);

        if let Some(output) = output {
            output.present();
        } else {
            self.output_texture = Some(view);
        }
    }

    pub fn render_to_texture(&mut self, texture: Option<Arc<RefCell<TextureView>>>) {
        self.output_texture = texture;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn new(position: [f32; 2], color: [f32; 4]) -> Self {
        Self { position, color }
    }

    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewProjectionUniform {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

pub struct RectPipeline {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_buffer_format: wgpu::IndexFormat,
    pub model_uniform_buffer: Buffer,
    pub model_uniform_buffer_bind_group: BindGroup,
    pub view_projection_uniform_buffer: Buffer,
    pub view_projection_uniform_buffer_bind_group: BindGroup,
    pub render_pipeline: RenderPipeline,
}

impl RectPipeline {
    const INITIAL_RECT_COUNT: usize = 1;

    pub fn init(device: &Device, surface_config: &SurfaceConfiguration) -> Self {
        // TODO: Move into shader manager?
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("../resources/shaders/rect.wgsl").into()),
        });

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (std::mem::size_of::<Vertex>() * 4 * Self::INITIAL_RECT_COUNT as usize)
                as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Index Buffer"),
            size: (std::mem::size_of::<u16>() * 6 * Self::INITIAL_RECT_COUNT as usize)
                as BufferAddress,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer_format = wgpu::IndexFormat::Uint16;

        let model_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Model Uniform Buffer"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let view_projection_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("View Projection Uniform Buffer"),
            size: std::mem::size_of::<ViewProjectionUniform>() as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let model_uniform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Model bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let model_uniform_buffer_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Model bind group"),
            layout: &model_uniform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: model_uniform_buffer.as_entire_binding(),
            }],
        });

        let view_projection_uniform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("View Projection bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let view_projection_uniform_buffer_bind_group =
            device.create_bind_group(&BindGroupDescriptor {
                label: Some("View Projection bind group"),
                layout: &view_projection_uniform_bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_projection_uniform_buffer.as_entire_binding(),
                }],
            });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &model_uniform_bind_group_layout,
                &view_projection_uniform_bind_group_layout,
            ],
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

        RectPipeline {
            vertex_buffer,
            index_buffer,
            index_buffer_format,
            model_uniform_buffer,
            model_uniform_buffer_bind_group,
            view_projection_uniform_buffer,
            view_projection_uniform_buffer_bind_group,
            render_pipeline,
        }
    }
}

// TODO: This needs to have coords and size specified in pixels/world coords.
// TODO: Split into relevant components.
// TODO: Set origin and ensure TRS happens in relation to it.
pub struct Rect {
    pub position: [f32; 2],
    pub color: [f32; 4],

    pub scale: [f32; 2],
    pub rotation_degrees: f32,
}

impl Rect {
    pub fn new(position: [f32; 2], color: [f32; 4]) -> Self {
        let scale = [1.0, 1.0];
        let rotation_degrees = 0.0;

        Self {
            position,
            color,
            scale,
            rotation_degrees,
        }
    }

    pub fn scale_rotation_translation(&self) -> Mat4 {
        let mut model = Mat4::from_translation(Vec3::new(self.position[0], self.position[1], 0.0));

        model *= Mat4::from_translation(Vec3::new(0.5 * self.scale[0], 0.5 * self.scale[1], 0.0));
        model *= Mat4::from_rotation_z(-self.rotation_degrees.to_radians());
        model *= Mat4::from_translation(Vec3::new(-0.5 * self.scale[0], -0.5 * self.scale[1], 0.0));

        model *= Mat4::from_scale(Vec3::new(self.scale[0], self.scale[1], 1.0));

        model
    }
}
