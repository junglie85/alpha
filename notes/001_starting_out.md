# Starting out

I've been experimenting with various Rust crates to build a game engine and editor.
I've hit a few roadblocks and learned some things that do not work as well as some that do.
Although I develop software for a living and take pride in my craft, I found I was forgetting everything I know and
trying to be too clever and complex.
This hobby project ill hopefully allow me to go back to basics, write something that works and let the usage drive the API.

## Prototype API overview

I implemented a very high level prototype API using `println` to illustrate each stage of the game loop so that I could get
a feel for how the editor, game and engine would be wired together.
The following code is here as a reminder of what that looked like and for me to refer back to in the future should I go astray.

In essence:

1. `Engine` runs an `Application`.
2. `Game` implements `Application` and a _game launcher_ will be used as the entrypoint to launch it via the `Engine`.
3. `Editor` implements `Application` and a _editor launcher_ will be used as the entrypoint to launch it via the `Engine`.
4. Common dependencies such as the `Renderer` will be passed into the `Application`.
    1. This allows the `Engine` to create the necessary systems for the `Game` and `Editor`.
    2. The `Editor` can create and call into an instance of `Game` with its resources.
    3. Hooks in the `Renderer` can be used to configure whether rendering is to the screen or a buffer that will be displayed in the `Editor`.
    4. Hooks in the `Game` can be used to configure whether all or a subset of systems will be run, useful for editing or playing in the `Editor`.

## The code

The heavy lifting of the prototype API is implemented in `lib.rs`:

```rust
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
```

The launcher for the game is a simple `main` function in `bin/alpha_game.rs`:

```rust
use alpha::{Engine, Game};

fn main() {
    let game = Game::default();
    let mut engine = Engine::default();
    engine.run(game);
}
```

The launcher for the editor is similar in `bin/alpha_editor.rs`:

```rust
use alpha::{Editor, Engine};

fn main() {
    let editor = Editor::default();
    let mut engine = Engine::default();
    engine.run(editor);
}
```

## The output

It's not much to look at but the output of `cargo run --bin alpha_game` is pretty simple:

```text
GAME on_start
GAME on_update - running
Rendering GAME to screen
GAME on_update - running
Rendering GAME to screen
GAME on_update - running
Rendering GAME to screen
GAME on_update - running
Rendering GAME to screen
GAME on_update - running
Rendering GAME to screen
GAME on_stop
```

It shows each of the lifecycle hooks for `Application` being called, that the game is running and rendering is to the screen.

The editor output, by running `cargo run --bin alpha_editor` is similar, but requires a bit more scrutiny:

```text
EDITOR on_start
GAME on_start
EDITOR on_update
GAME on_update - paused
Rendering GAME to buffer
Rendering EDITOR to screen
EDITOR on_update
Simulate start playing game in the editor
GAME on_update - running
Rendering GAME to buffer
Rendering EDITOR to screen
EDITOR on_update
GAME on_update - running
Rendering GAME to buffer
Rendering EDITOR to screen
EDITOR on_update
Simulate stop playing game in the editor
GAME on_update - paused
Rendering GAME to buffer
Rendering EDITOR to screen
EDITOR on_update
GAME on_update - paused
Rendering GAME to buffer
Rendering EDITOR to screen
GAME on_stop
EDITOR on_stop
```

It shows each of the lifecycle hooks for `Application` being called for the editor, with the editor calling into the game.
The game can be seen rendering to a buffer and the editor rendering to the screen.
It also shows being able to control whether the game is paused or playing from within the editor.

## Wrap up

Although this is a pretty text heavy output, I'm pleased that the prototype API shows how the same `Engine` can be used
to run both an `Editor` and a `Game` by implementing the `Application` trait.
This API is likely to change significantly over time, especially as I learn to grapple with the APIs of various crates such as
`winit`, `wgpu` and `egui`, which will be the main foundations of the engine.
However, I hope this has set me up for success as I move on to the next steps - actually drawing something to the screen.

[View the code](https://github.com/junglie85/alpha/tree/341314716278282684e31e154baf0a1f8e1c8f04) on GitHub.
