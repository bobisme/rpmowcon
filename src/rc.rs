use micromath::F32Ext;

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub enum Button {
    Pressed,
    #[default]
    Released,
}

impl Button {
    pub fn from_channel(val: u16) -> Self {
        match val {
            x if x > 1000 => Self::Pressed,
            _ => Self::Released,
        }
    }
}

#[derive(Clone, Copy, Debug, defmt::Format, Default)]
pub struct Axis {
    /// From -1.0 to 1.0
    val: f32,
}

impl Axis {
    pub fn new(val: f32) -> Self {
        Self { val }
    }

    pub fn from_channel(val: u16) -> Self {
        let val = (val as f32 - 1000.0) / 800.0;
        let val = match val {
            x if x.abs() < 0.02 => 0.0,
            x => x.clamp(-1.0, 1.0),
        };
        Self { val }
    }
}

impl PartialEq for Axis {
    fn eq(&self, other: &Axis) -> bool {
        (self.val - other.val).abs() <= f32::EPSILON
    }
}

impl Eq for Axis {}

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub struct Stick {
    x: Axis,
    y: Axis,
}

impl Stick {
    pub fn new(x: Axis, y: Axis) -> Self {
        Self { x, y }
    }

    pub fn from_channels(x: u16, y: u16) -> Self {
        Self {
            x: Axis::from_channel(x),
            y: Axis::from_channel(y),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub enum ThreeWay {
    Up,
    #[default]
    Mid,
    Down,
}

impl ThreeWay {
    pub fn from_channel(val: u16) -> Self {
        match val {
            1800 => Self::Up,
            1000 => Self::Mid,
            200 => Self::Down,
            _ => Self::Mid,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub struct RadioLinkController {
    left_thumb: Stick,
    left_shoulder: Axis,
    left_trigger: ThreeWay,
    right_thumb: Stick,
    right_shoulder: Button,
    right_trigger: ThreeWay,
}

impl RadioLinkController {
    pub fn from_channels(chs: &[crate::sbus::Chan; 16]) -> Self {
        Self {
            right_thumb: Stick::from_channels(chs[0].get(), chs[1].get()),
            left_thumb: Stick::from_channels(chs[3].get(), chs[2].get()),
            right_trigger: ThreeWay::from_channel(chs[4].get()),
            right_shoulder: Button::from_channel(chs[5].get()),
            left_trigger: ThreeWay::from_channel(chs[6].get()),
            left_shoulder: Axis::from_channel(chs[7].get()),
        }
    }
}
