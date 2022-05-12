use crate::error::Error;
use crate::logging;
use crate::renderer::Renderer;

pub trait Application {
    fn on_start(&mut self);

    fn on_update(&mut self, renderer: &mut Renderer);

    fn on_stop(&mut self);
}

#[derive(Default)]
pub struct Engine {
    frames: usize,
}

impl Engine {
    pub fn init() -> Result<Self, Error> {
        logging::init("info")?;

        Ok(Engine::default())
    }

    pub fn run(&mut self, mut app: impl Application) -> Result<(), Error> {
        let mut renderer = Renderer::default();

        app.on_start();

        while self.frames < 5 {
            app.on_update(&mut renderer);
            self.frames += 1;
        }

        app.on_stop();

        Ok(())
    }
}
