mod mask;

use mask::JoyMask;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct JoyState(u8);

#[allow(dead_code)]
impl JoyState {
    pub fn new(state: u8) -> Self {
        Self(state)
    }
    
    pub fn is_up(&self) -> bool {
        self.0 & (u8::from(JoyMask::Up)) == 0
    }

    pub fn is_down(&self) -> bool {
        self.0 & (u8::from(JoyMask::Down)) == 0
    }

    pub fn is_left(&self) -> bool {
        self.0 & (u8::from(JoyMask::Left)) == 0
    }

    pub fn is_right(&self) -> bool {
        self.0 & (u8::from(JoyMask::Right)) == 0
    }

    pub fn is_fire(&self) -> bool {
        self.0 & (u8::from(JoyMask::Fire)) == 0
    }
}
