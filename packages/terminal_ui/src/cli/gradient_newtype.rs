use colorgrad;
use tui::{
    style::{Color},
};

pub struct NewGradient(Color);

impl From<colorgrad::Color> for NewGradient {
    fn from(color: colorgrad::Color) -> Self {
        let c = color.to_rgba8();
        let new_c = Color::Rgb(c[0], c[1], c[2]);
        NewGradient(new_c)
    }
}

impl Into<Color> for NewGradient {
    fn into(self) -> Color {
        self.0
    }
}
