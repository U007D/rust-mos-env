mod mask;

use mask::JoystickMask;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct JoystickState(u8);

#[allow(dead_code)]
impl JoystickState {
    pub fn new(state: u8) -> Option<Self> {
        // SAFETY:
        // `is_valid_state()` verifies `state` contains only valid bits and only makes
        // the (`unsafe`) call to `new_unchecked()` if valid.  This ensures the
        // call is sound.
        Self::is_valid_state(state).then(|| unsafe { Self::new_unchecked(state) })
    }

    // SAFETY:
    // Only various combinations of the bits defined by `JoystickMask` are valid
    // states.  Undefined behavior will result if `new_unchecked()` is called
    // with invalid input.  Call `new` for proper validation of `state`.
    pub unsafe fn new_unchecked(state: u8) -> Self { JoystickState(state) }

    pub fn is_up(&self) -> bool { self.0 & u8::from(JoystickMask::Up) == 0 }

    pub fn is_down(&self) -> bool { self.0 & u8::from(JoystickMask::Down) == 0 }

    pub fn is_left(&self) -> bool { self.0 & u8::from(JoystickMask::Left) == 0 }

    pub fn is_right(&self) -> bool { self.0 & u8::from(JoystickMask::Right) == 0 }

    pub fn is_fire(&self) -> bool { self.0 & u8::from(JoystickMask::Fire) == 0 }

    pub fn is_idle(&self) -> bool {
        use JoystickMask as Jm;
        self.0
            & (u8::from(Jm::Up)
                | u8::from(Jm::Down)
                | u8::from(Jm::Left)
                | u8::from(Jm::Right)
                | u8::from(Jm::Fire))
            == 0
    }

    #[inline]
    fn is_valid_state(state: u8) -> bool {
        use JoystickMask as Jm;

        [
            // Simple states
            u8::from(Jm::Up),
            u8::from(Jm::Down),
            u8::from(Jm::Left),
            u8::from(Jm::Right),
            u8::from(Jm::Fire),
            // Valid bi-states (only some are valid--e.g. `JoystickMask::Up` and
            // `JoystickMask::Down` cannot both be `true` simultaneously
            u8::from(Jm::Up) | u8::from(Jm::Fire),
            u8::from(Jm::Down) | u8::from(Jm::Fire),
            u8::from(Jm::Left) | u8::from(Jm::Fire),
            u8::from(Jm::Right) | u8::from(Jm::Fire),
            // NE
            u8::from(Jm::Up) | u8::from(Jm::Right),
            // SE
            u8::from(Jm::Right) | u8::from(Jm::Down),
            // SW
            u8::from(Jm::Down) | u8::from(Jm::Left),
            // NW
            u8::from(Jm::Left) | u8::from(Jm::Up),
            // Valid tri-states
            // NE + Fire
            u8::from(Jm::Up) | u8::from(Jm::Right) | u8::from(Jm::Fire),
            // SE + Fire
            u8::from(Jm::Right) | u8::from(Jm::Down) | u8::from(Jm::Fire),
            // SW + Fire
            u8::from(Jm::Down) | u8::from(Jm::Left) | u8::from(Jm::Fire),
            // NW + Fire
            u8::from(Jm::Left) | u8::from(Jm::Up) | u8::from(Jm::Fire),
        ]
        .contains(&state)
    }
}
