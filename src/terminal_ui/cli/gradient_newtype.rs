use colorgrad;
use gradient_tui_fork::{
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

impl From<NewGradient> for Color {
    fn from(val: NewGradient) -> Self {
        val.0
    }
}
