#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Song {
    MagicalSoundShower = 0,
    SplashWave = 1,
}

impl From<Song> for u8 {
    fn from(song: Song) -> Self {
        use Song as S;
        match song {
            S::MagicalSoundShower => 0,
            S::SplashWave => 1,
        }
    }
}
