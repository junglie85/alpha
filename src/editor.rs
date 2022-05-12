use crate::engine::Application;
use crate::game::Game;
use crate::renderer::Renderer;
use log::info;
use std::cell::RefCell;
use std::sync::Arc;

pub trait EditorApplication: Application + Pause {}

pub trait Pause {
    fn pause(&mut self, paused: bool);
}

#[derive(Default)]
pub struct Editor {
    game: Option<Arc<RefCell<dyn EditorApplication>>>,
    frames: usize,
}

impl Application for Editor {
    fn on_start(&mut self) {
        info!("EDITOR on_start");

        let mut game = Game::default();
        game.on_start();
        game.pause(true);
        self.game = Some(Arc::new(RefCell::new(game)));
    }

    fn on_update(&mut self, renderer: &mut Renderer) {
        info!("EDITOR on_update");

        let mut game = self.game.as_ref().unwrap().borrow_mut();

        let play_game = match self.frames {
            1 => {
                info!("Simulate start playing game in the editor");
                true
            }
            2 => true,
            3 => {
                info!("Simulate stop playing game in the editor");
                false
            }
            _ => false,
        };

        renderer.render_to_screen(false);
        game.pause(!play_game);
        game.on_update(renderer);
        renderer.render_to_screen(true);

        renderer.render("EDITOR");

        self.frames += 1;
    }

    fn on_stop(&mut self) {
        let mut game = self.game.as_ref().unwrap().borrow_mut();
        game.on_stop();

        info!("EDITOR on_stop");
    }
}
