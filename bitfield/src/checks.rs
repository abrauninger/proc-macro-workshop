pub trait TotalSizeIsMultipleOfEightBits {}

pub struct ZeroMod8 {}
pub struct OneMod8 {}
pub struct TwoMod8 {}
pub struct ThreeMod8 {}
pub struct FourMod8 {}
pub struct FiveMod8 {}
pub struct SixMod8 {}
pub struct SevenMod8 {}

impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {}

pub trait Mod8Check {
    type Type;
}

pub struct Mod8<const MOD8: usize> {}

impl Mod8Check for Mod8<0> {
    type Type = ZeroMod8;
}

impl Mod8Check for Mod8<1> {
    type Type = OneMod8;
}

impl Mod8Check for Mod8<2> {
    type Type = TwoMod8;
}

impl Mod8Check for Mod8<3> {
    type Type = ThreeMod8;
}

impl Mod8Check for Mod8<4> {
    type Type = FourMod8;
}

impl Mod8Check for Mod8<5> {
    type Type = FiveMod8;
}

impl Mod8Check for Mod8<6> {
    type Type = SixMod8;
}

impl Mod8Check for Mod8<7> {
    type Type = SevenMod8;
}