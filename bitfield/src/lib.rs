pub mod checks;
pub mod field_data;

// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub use bitfield_impl::bitfield;
pub use bitfield_impl::BitfieldSpecifier;
use bitfield_impl::gen_bit_width_types;

pub trait Specifier {
    const BITS: usize;
    type ACCESSOR;
}

gen_bit_width_types!(1..=64);

impl Specifier for bool {
    const BITS: usize = 1;
    type ACCESSOR = bool;
}

pub trait Serialize<const SIZE: usize> {
    type Type;

    fn serialize(t: Self::Type) -> [u8; SIZE];
    fn deserialize(bytes: [u8; SIZE]) -> Self::Type;
}

impl Serialize<1> for bool {
    type Type = bool;

    fn serialize(t: bool) -> [u8; 1] {
        [t as u8]
    }

    fn deserialize(bytes: [u8; 1]) -> bool {
        bytes[0] != 0
    }
}

impl Serialize<1> for u8 {
    type Type = u8;

    fn serialize(t: u8) -> [u8; 1] {
        [t]
    }

    fn deserialize(bytes: [u8; 1]) -> u8 {
        bytes[0]
    }
}

impl Serialize<2> for u16 {
    type Type = u16;

    fn serialize(t: u16) -> [u8; 2] {
        t.to_le_bytes()
    }

    fn deserialize(bytes: [u8; 2]) -> u16 {
        u16::from_le_bytes(bytes)
    }
}

impl Serialize<4> for u32 {
    type Type = u32;

    fn serialize(t: u32) -> [u8; 4] {
        t.to_le_bytes()
    }

    fn deserialize(bytes: [u8; 4]) -> u32 {
        u32::from_le_bytes(bytes)
    }
}

impl Serialize<8> for u64 {
    type Type = u64;

    fn serialize(t: u64) -> [u8; 8] {
        t.to_le_bytes()
    }

    fn deserialize(bytes: [u8; 8]) -> u64 {
        u64::from_le_bytes(bytes)
    }
}