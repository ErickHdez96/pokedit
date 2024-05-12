#[path = "../app.rs"]
mod app;

use std::{convert::Infallible, time::Duration};

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Size},
    pixelcolor::Rgb888,
    primitives::Rectangle,
    Pixel,
};
use embedded_graphics_simulator::{OutputSettings, SimulatorDisplay, SimulatorEvent, Window};
use log::info;
use pokedit::{parse_args, BinaryConfig};
use sdl2::keyboard::Keycode;

use app::{
    input::{Key, KeyEvent},
    App, Platform,
};

type Display = SimulatorDisplay<Rgb888>;

const HELP_STR: &str = "
A pokemon save file editor

Usage: pokedit [OPTIONS] FILE

Arguments:
    FILE\tPokemon save file to edit.
";

struct SimulatorPlatform {
    window: Window,
    display: Display,
}

impl SimulatorPlatform {
    const DISPLAY_WIDTH: u32 = 640;
    const DISPLAY_HEIGHT: u32 = 480;

    pub fn new() -> Self {
        Self {
            window: Window::new("Pokedit", &OutputSettings::default()),
            display: Display::new(Size::new(Self::DISPLAY_WIDTH, Self::DISPLAY_HEIGHT)),
        }
    }
}

impl Platform for SimulatorPlatform {
    fn flush(&mut self) {
        self.window.update(&self.display);
    }

    fn display_width(&self) -> u32 {
        Self::DISPLAY_WIDTH
    }

    fn display_height(&self) -> u32 {
        Self::DISPLAY_HEIGHT
    }

    async fn poll(&mut self) -> KeyEvent {
        loop {
            let Some(event) = self.window.events().next() else {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            };

            match event {
                SimulatorEvent::KeyDown {
                    keycode, repeat, ..
                } => {
                    if keycode == Keycode::Q {
                        return KeyEvent::Pressed(Key::Quit);
                    }

                    return if repeat {
                        KeyEvent::Autorepeat(Key::from(keycode))
                    } else {
                        KeyEvent::Pressed(Key::from(keycode))
                    };
                }
                SimulatorEvent::KeyUp { keycode, .. } => {
                    return KeyEvent::Released(Key::from(keycode));
                }
                SimulatorEvent::Quit => {
                    return KeyEvent::Pressed(Key::Quit);
                }
                _ => continue,
            }
        }
    }
}

impl Dimensions for SimulatorPlatform {
    fn bounding_box(&self) -> Rectangle {
        self.display.bounding_box()
    }
}

impl DrawTarget for SimulatorPlatform {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_env().unwrap();

    let args = parse_args(BinaryConfig {
        help: HELP_STR.trim(),
    });
    let mut app = App::new(SimulatorPlatform::new());
    if let Some(save_file_path) = args.input {
        let bkp = save_file_path.with_extension("bkp");
        if !bkp.exists() {
            info!("Creating backup on {:#?}", bkp);
            std::fs::copy(&save_file_path, bkp)?;
        }
        app.open(save_file_path)?;
    }

    info!("Running pokedit");
    app.run_event_loop().await?;
    info!("Goodbye!");

    Ok(())
}
