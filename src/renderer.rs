use crate::error::Error;
use crate::renderer::camera::Camera;
use crate::renderer::rect::{Rect, RectPipeline, Vertex, ViewProjectionUniform};
use bytemuck::cast_slice;
use egui::epaint::ClippedShape;
use egui::CtxRef;
use egui_wgpu_backend::ScreenDescriptor;
use glam::{Mat4, Vec4, Vec4Swizzles};
use log::info;
use std::cell::RefCell;
use std::sync::Arc;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Adapter, BufferAddress, BufferUsages, CommandEncoder, Device, Instance, Queue, Surface,
    SurfaceConfiguration, SurfaceTexture, Texture, TextureView,
};
use winit::window::Window;

pub mod camera;
pub mod rect;

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
    egui_render_pass: egui_wgpu_backend::RenderPass,
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

        let egui_render_pass =
            egui_wgpu_backend::RenderPass::new(&device, surface_config.format, 1);

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
            egui_render_pass,
        }
    }

    pub fn prepare(&mut self) -> RenderContext {
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

        RenderContext { output, view }
    }

    pub fn begin_scene(&mut self, camera: &Camera) -> Scene {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let view_projection_uniform = ViewProjectionUniform {
            view: camera.get_view().to_cols_array_2d(),
            projection: camera.get_projection().to_cols_array_2d(),
        };

        let view_projection_uniform_buffer =
            self.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("View Projection Uniform Buffer"),
                contents: cast_slice(&[view_projection_uniform]),
                usage: BufferUsages::COPY_SRC,
            });

        encoder.copy_buffer_to_buffer(
            &view_projection_uniform_buffer,
            0,
            &self.rect_pipeline.view_projection_uniform_buffer, // TODO: Should this uniform buffer be tied to specific pipeline?
            0,
            std::mem::size_of::<ViewProjectionUniform>() as BufferAddress,
        );

        let vertices = Vec::new();

        let indices = Vec::new();

        let transform = Mat4::IDENTITY;

        let index_offset = 0;

        Scene {
            encoder,
            vertices,
            indices,
            transform,
            index_offset,
        }
    }

    pub fn end_scene(&mut self, mut scene: Scene, ctx: &mut RenderContext) {
        let vertex_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(&scene.vertices),
            usage: BufferUsages::COPY_SRC,
        });

        let index_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: cast_slice(&scene.indices),
            usage: BufferUsages::COPY_SRC,
        });

        if scene.vertices.len() > self.rect_pipeline.max_vertices
            || scene.indices.len() > self.rect_pipeline.max_indices
        {
            self.rect_pipeline.resize_buffers(
                self.device.as_ref(),
                scene.vertices.len(),
                scene.indices.len(),
            )
        }

        scene.encoder.copy_buffer_to_buffer(
            &vertex_buffer,
            0,
            &self.rect_pipeline.vertex_buffer, // TODO: How do we know what pipeline to use here?
            0,
            (std::mem::size_of::<Vertex>() * scene.vertices.len()) as BufferAddress,
        );

        scene.encoder.copy_buffer_to_buffer(
            &index_buffer,
            0,
            &self.rect_pipeline.index_buffer,
            0,
            (std::mem::size_of::<u16>() * scene.indices.len()) as BufferAddress,
        );

        {
            let view = &ctx.view.as_ref().borrow();
            let mut render_pass = scene
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
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
            render_pass.set_bind_group(0, &self.rect_pipeline.uniforms_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.rect_pipeline.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.rect_pipeline.index_buffer.slice(..),
                self.rect_pipeline.index_buffer_format,
            );
            render_pass.draw_indexed(0..scene.indices.len() as u32, 0, 0..1);
        }
        let command_buffers = vec![scene.encoder.finish()];
        let command_buffers = command_buffers;

        self.queue.submit(command_buffers);
    }

    pub fn finalise(&mut self, ctx: RenderContext) {
        if let Some(output) = ctx.output {
            output.present();
        } else {
            self.output_texture = Some(ctx.view);
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

    pub fn draw_rect(&mut self, scene: &mut Scene, rect: &Rect) {
        let transform = rect.scale_rotation_translation();
        let vertices: Vec<Vertex> = Rect::VERTEX_COORDS
            .iter()
            .map(|vc| {
                let position = transform.mul_vec4(Vec4::from((vc[0], vc[1], 0.0, 1.0)));
                let color = rect.color;
                Vertex::new(position.xy().to_array(), color.to_array())
            })
            .collect();

        scene.vertices.extend_from_slice(&vertices);

        let indices: Vec<u16> = Rect::INDICES
            .iter()
            .map(|i| i + scene.index_offset)
            .collect();

        scene.indices.extend_from_slice(&indices);
        scene.index_offset += 4;
    }

    pub fn render_to_texture(&mut self, texture: Option<Arc<RefCell<TextureView>>>) {
        self.output_texture = texture;
    }

    pub fn egui_texture_from_wgpu_texture(&mut self, texture: &Texture) -> egui::TextureId {
        egui_wgpu_backend::RenderPass::egui_texture_from_wgpu_texture(
            &mut self.egui_render_pass,
            &self.device,
            texture,
            wgpu::FilterMode::Linear,
        )
    }

    pub fn begin_egui(
        &mut self,
        ctx: &RenderContext,
        egui_ctx: CtxRef,
        paint_commands: Vec<ClippedShape>,
    ) {
        let paint_jobs = egui_ctx.tessellate(paint_commands);
        let font_image = egui_ctx.font_image();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let screen_descriptor = ScreenDescriptor {
                physical_width: self.surface_config.width,
                physical_height: self.surface_config.height,
                scale_factor: self.scale_factor as f32,
            };

            self.egui_render_pass
                .update_texture(&self.device, &self.queue, &font_image);
            self.egui_render_pass
                .update_user_textures(&self.device, &self.queue);
            self.egui_render_pass.update_buffers(
                &self.device,
                &self.queue,
                &paint_jobs,
                &screen_descriptor,
            );

            self.egui_render_pass
                .execute(
                    &mut encoder,
                    &ctx.view.borrow(),
                    &paint_jobs,
                    &screen_descriptor,
                    Some(wgpu::Color::BLACK),
                )
                .unwrap();
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

pub struct RenderContext {
    pub output: Option<SurfaceTexture>,
    pub view: Arc<RefCell<TextureView>>,
}

pub struct Scene {
    pub encoder: CommandEncoder,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub transform: Mat4,
    pub index_offset: u16,
}
