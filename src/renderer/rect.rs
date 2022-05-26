use crate::components::{compute_transformation_matrix, Transform};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec4};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, Device, Face, FragmentState,
    FrontFace, IndexFormat, MultisampleState, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, SurfaceConfiguration, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

// TODO: This needs to have coords and size specified in pixels/world coords.
// TODO: Split into relevant components.
// TODO: Set origin and ensure TRS happens in relation to it.
pub struct Rect {
    pub color: Vec4,
    pub position: Vec2,
    pub rotation_degrees: f32,
    pub size: Vec2,
}

impl Rect {
    pub const VERTEX_COORDS: [[f32; 2]; 4] = [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]];

    #[rustfmt::skip]
    pub const INDICES: [u16; 6] = [
        0, 1, 2,
        0, 2, 3
    ];

    pub fn new(position: Vec2, rotation_degrees: f32, size: Vec2, color: Vec4) -> Self {
        Self {
            color,
            position,
            rotation_degrees,
            size,
        }
    }

    pub fn scale_rotation_translation(&self) -> Mat4 {
        // TODO: All transformations in relation to origin.
        let t = Transform {
            position: self.position,
            size: self.size,
            rotation: self.rotation_degrees,
        };
        compute_transformation_matrix(&t)
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
    pub max_vertices: usize,
    pub max_indices: usize,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_buffer_format: IndexFormat,
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

        let max_vertices = 4 * Self::INITIAL_RECT_COUNT;
        let max_indices = 6 * Self::INITIAL_RECT_COUNT;

        let (vertex_buffer, index_buffer) = Self::create_buffers(device, max_vertices, max_indices);

        let index_buffer_format = wgpu::IndexFormat::Uint16;

        let view_projection_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("View Projection Uniform Buffer"),
            size: std::mem::size_of::<ViewProjectionUniform>() as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniforms_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Uniforms Bind Group Layout"),
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

        let uniforms_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Uniforms Bind Group"),
            layout: &uniforms_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_projection_uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniforms_bind_group_layout],
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
            max_vertices,
            max_indices,
            vertex_buffer,
            index_buffer,
            index_buffer_format,
            uniforms_bind_group,
            view_projection_uniform_buffer,
            render_pipeline,
        }
    }

    pub fn resize_buffers(&mut self, device: &Device, vertex_count: usize, index_count: usize) {
        let (vertex_buffer, index_buffer) = Self::create_buffers(device, vertex_count, index_count);

        self.vertex_buffer = vertex_buffer;
        self.index_buffer = index_buffer;
    }

    fn create_buffers(
        device: &Device,
        vertex_count: usize,
        index_count: usize,
    ) -> (Buffer, Buffer) {
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (std::mem::size_of::<Vertex>() * vertex_count) as BufferAddress,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Index Buffer"),
            size: (std::mem::size_of::<u16>() * index_count) as BufferAddress,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        (vertex_buffer, index_buffer)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ViewProjectionUniform {
    pub(crate) view: [[f32; 4]; 4],
    pub(crate) projection: [[f32; 4]; 4],
}
