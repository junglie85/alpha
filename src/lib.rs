use std::cell::RefCell;
use std::sync::Arc;

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
            println!("Rendering {msg} to screen");
        } else {
            println!("Rendering {msg} to buffer");
        }
    }

    pub fn render_to_screen(&mut self, output_to_screen: bool) {
        self.output_to_screen = output_to_screen
    }
}

pub trait Application {
    fn on_start(&mut self);

    fn on_update(&mut self, renderer: &mut Renderer);

    fn on_stop(&mut self);
}

pub trait Pause {
    fn pause(&mut self, paused: bool);
}

#[derive(Default)]
pub struct Engine {
    frames: usize,
}

impl Engine {
    pub fn run(&mut self, mut app: impl Application) {
        let mut renderer = Renderer::default();

        app.on_start();

        while self.frames < 5 {
            app.on_update(&mut renderer);
            self.frames += 1;
        }

        app.on_stop();
    }
}

trait EditorApplication: Application + Pause {}

#[derive(Default)]
pub struct Editor {
    game: Option<Arc<RefCell<dyn EditorApplication>>>,
    frames: usize,
}

impl Application for Editor {
    fn on_start(&mut self) {
        println!("EDITOR on_start");

        let mut game = Game::default();
        game.on_start();
        game.pause(true);
        self.game = Some(Arc::new(RefCell::new(game)));
    }

    fn on_update(&mut self, renderer: &mut Renderer) {
        println!("EDITOR on_update");

        let mut game = self.game.as_ref().unwrap().borrow_mut();

        let play_game = match self.frames {
            1 => {
                println!("Simulate start playing game in the editor");
                true
            }
            2 => true,
            3 => {
                println!("Simulate stop playing game in the editor");
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

        println!("EDITOR on_stop");
    }
}

#[derive(Default)]
pub struct Game {
    paused: bool,
}

impl Application for Game {
    fn on_start(&mut self) {
        println!("GAME on_start");
    }

    fn on_update(&mut self, renderer: &mut Renderer) {
        let paused_or_running = if self.paused { "paused" } else { "running" };
        println!("GAME on_update - {}", paused_or_running);
        renderer.render("GAME");
    }

    fn on_stop(&mut self) {
        println!("GAME on_stop");
    }
}

impl Pause for Game {
    fn pause(&mut self, paused: bool) {
        self.paused = paused;
    }
}

impl EditorApplication for Game {}
