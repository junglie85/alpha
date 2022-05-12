use crate::error::Error;
use log::info;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

pub fn init() -> Result<(EventLoop<()>, Window, WinitInputHelper), Error> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Alpha Engine")
        .build(&event_loop)
        .unwrap();
    let input = WinitInputHelper::new();

    info!("platform initialised");

    Ok((event_loop, window, input))
}
