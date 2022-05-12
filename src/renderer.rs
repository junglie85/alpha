use log::info;

pub struct Renderer {
    output_to_screen: bool,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            output_to_screen: true,
        }
    }
}

impl Renderer {
    pub fn render(&self, msg: &str) {
        if self.output_to_screen {
            info!("Rendering {msg} to screen");
        } else {
            info!("Rendering {msg} to buffer");
        }
    }

    pub fn render_to_screen(&mut self, output_to_screen: bool) {
        self.output_to_screen = output_to_screen
    }
}
