use crate::renderer::camera::Camera;
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{
    util::BufferInitDescriptor, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, Buffer,
    BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState,
    ColorWrites, CommandBuffer, Device, Face, FragmentState, FrontFace, IndexFormat,
    MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    SurfaceConfiguration, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

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
    const VERTEX_COORDS: [[f32; 2]; 4] = [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]];

    #[rustfmt::skip]
    const INDICES: [u16; 6] = [
        0, 1, 2,
        0, 2, 3
    ];

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

    pub fn vertices(&self) -> [Vertex; 4] {
        [
            Vertex::new(Self::VERTEX_COORDS[0], self.color),
            Vertex::new(Self::VERTEX_COORDS[1], self.color),
            Vertex::new(Self::VERTEX_COORDS[2], self.color),
            Vertex::new(Self::VERTEX_COORDS[3], self.color),
        ]
    }

    pub fn indices(&self) -> &[u16] {
        &Self::INDICES
    }

    pub fn scale_rotation_translation(&self) -> Mat4 {
        let mut transform =
            Mat4::from_translation(Vec3::new(self.position[0], self.position[1], 0.0));

        transform *=
            Mat4::from_translation(Vec3::new(0.5 * self.scale[0], 0.5 * self.scale[1], 0.0));
        transform *= Mat4::from_rotation_z(-self.rotation_degrees.to_radians());
        transform *=
            Mat4::from_translation(Vec3::new(-0.5 * self.scale[0], -0.5 * self.scale[1], 0.0));

        transform *= Mat4::from_scale(Vec3::new(self.scale[0], self.scale[1], 1.0));

        transform
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

pub struct RectPipeline {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_buffer_format: IndexFormat,
    pub transform_uniform_buffer: Buffer,
    pub view_projection_uniform_buffer: Buffer,
    pub uniforms_bind_group: BindGroup,
    pub render_pipeline: RenderPipeline,
}

impl RectPipeline {
    const INITIAL_RECT_COUNT: usize = 1;

    pub fn init(device: &Device, surface_config: &SurfaceConfiguration) -> Self {
        // TODO: Move into shader manager?
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("../../resources/shaders/rect.wgsl").into()),
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

        let transform_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Transform Uniform Buffer"),
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

        let uniforms_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Uniforms Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let uniforms_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Uniforms Bind Group"),
            layout: &uniforms_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: transform_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: view_projection_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &uniforms_bind_group_layout,
                // &view_projection_uniform_bind_group_layout,
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
            transform_uniform_buffer,
            uniforms_bind_group,
            view_projection_uniform_buffer,
            render_pipeline,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct ViewProjectionUniform {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

pub fn draw_rect(
    rect: &Rect,
    camera: &Camera,
    device: &Device,
    rect_pipeline: &RectPipeline,
    view: &TextureView,
) -> Vec<CommandBuffer> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    let view_projection_uniform = ViewProjectionUniform {
        view: camera.get_view().to_cols_array_2d(),
        projection: camera.get_projection().to_cols_array_2d(),
    };

    let vertices = &rect.vertices();
    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: cast_slice(vertices),
        usage: BufferUsages::COPY_SRC,
    });

    let indices = rect.indices();
    let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: cast_slice(indices),
        usage: BufferUsages::COPY_SRC,
    });

    let transform_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Transform Uniform Buffer"),
        contents: cast_slice(rect.scale_rotation_translation().as_ref()),
        usage: BufferUsages::COPY_SRC,
    });

    let view_projection_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("View Projection Uniform Buffer"),
        contents: cast_slice(&[view_projection_uniform]),
        usage: BufferUsages::COPY_SRC,
    });

    encoder.copy_buffer_to_buffer(
        &vertex_buffer,
        0,
        &rect_pipeline.vertex_buffer,
        0,
        (std::mem::size_of::<Vertex>() * vertices.len()) as BufferAddress,
    );

    encoder.copy_buffer_to_buffer(
        &index_buffer,
        0,
        &rect_pipeline.index_buffer,
        0,
        (std::mem::size_of::<u16>() * indices.len()) as BufferAddress,
    );

    encoder.copy_buffer_to_buffer(
        &transform_uniform_buffer,
        0,
        &rect_pipeline.transform_uniform_buffer,
        0,
        std::mem::size_of::<Mat4>() as BufferAddress,
    );

    encoder.copy_buffer_to_buffer(
        &view_projection_uniform_buffer,
        0,
        &rect_pipeline.view_projection_uniform_buffer,
        0,
        std::mem::size_of::<ViewProjectionUniform>() as BufferAddress,
    );

    {
        let view = &view;
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

        render_pass.set_pipeline(&rect_pipeline.render_pipeline);
        render_pass.set_bind_group(0, &rect_pipeline.uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, rect_pipeline.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            rect_pipeline.index_buffer.slice(..),
            rect_pipeline.index_buffer_format,
        );
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }

    let command_buffers = vec![encoder.finish()];

    command_buffers
}
