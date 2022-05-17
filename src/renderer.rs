use crate::error::Error;
use crate::renderer::camera::Camera;
use crate::renderer::rect::{Rect, RectPipeline};
use log::info;
use std::cell::RefCell;
use std::sync::Arc;
use wgpu::{Adapter, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureView};
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

    pub fn draw_rect(&mut self, rect: &Rect, camera: &Camera) {
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

        let command_buffers = rect::draw_rect(
            rect,
            camera,
            self.device.as_ref(),
            &self.rect_pipeline,
            &view.as_ref().borrow(),
        );

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
