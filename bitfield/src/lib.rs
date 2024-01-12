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
use bitfield_impl::gen_bit_width_types;

pub trait Specifier {
    const BITS: usize;
}

gen_bit_width_types!(1..=64);

// Helper functions
// TODO: Move these to a detail module?

/// Returns a byte consisting of 1's in the bits [bit_start_index .. bit_start_index + bit_count],
/// and 0's otherwise.
/// Note that the leftmost bit is the 0th bit index.
fn create_bit_mask(bit_start_index: usize, bit_count: usize) -> u8 {
    assert!(bit_start_index < 8);
    assert!(bit_start_index + bit_count <= 8);

    let mask_usize: usize = ((1 << bit_count) - 1) << (8 - bit_start_index - bit_count);
    mask_usize.try_into().unwrap()
}

pub fn copy_bits(
    source_data: &[u8],
    destination_data: &mut [u8],
    source_bit_start_index: usize,
    destination_bit_start_index: usize,
    bit_count: usize) {

    let mut current_destination_bit_index = destination_bit_start_index;
    let mut current_source_bit_index = source_bit_start_index;
    let mut remaining_bit_count = bit_count;

    while remaining_bit_count > 0 {
        let destination_chunk_bit_count = 8 - (current_destination_bit_index % 8);
        let source_chunk_bit_count = 8 - (current_source_bit_index % 8);

        let byte_aligned_chunk_bit_count = std::cmp::min(destination_chunk_bit_count, source_chunk_bit_count);
        let chunk_bit_count = std::cmp::min(remaining_bit_count, byte_aligned_chunk_bit_count);

        let destination_mask = create_bit_mask(current_destination_bit_index % 8, chunk_bit_count);
        let source_mask = create_bit_mask(current_source_bit_index % 8, chunk_bit_count);

        let bit_offset = (isize::try_from(current_destination_bit_index % 8).unwrap() - isize::try_from(current_source_bit_index % 8).unwrap()) % 8;
    
        let destination_byte = &mut destination_data[current_destination_bit_index / 8];
        *destination_byte = *destination_byte & !destination_mask;

        let source_byte = source_data[current_source_bit_index / 8];
        let source_data = source_byte & source_mask;

        let shifted_source_data =
            if bit_offset > 0 {
                source_data >> bit_offset
            } else {
                source_data << -bit_offset
            };

        *destination_byte = *destination_byte | shifted_source_data;

        current_destination_bit_index = current_destination_bit_index + chunk_bit_count;
        current_source_bit_index = current_source_bit_index + chunk_bit_count;
        remaining_bit_count = remaining_bit_count - chunk_bit_count;
    }
}

pub fn get_field_data<const FIELD_DATA_BYTE_COUNT: usize>(source_data: &[u8], bit_start_index: usize, bit_count: usize) -> [u8; FIELD_DATA_BYTE_COUNT] {
    let mut field_data: [u8; FIELD_DATA_BYTE_COUNT] = [0; FIELD_DATA_BYTE_COUNT];

    // We use little-endian byte ordering, so unused bytes in the field data should be at the end
    // of the field_data array.
    // We also want the right-most bit of the data to align with the right-most bit of the
    // last non-empty byte.
    let field_data_bit_start_index = ((FIELD_DATA_BYTE_COUNT * 8) - bit_count) % 8;

    copy_bits(
        &source_data,
        &mut field_data,
        bit_start_index /*source_bit_start_index*/,
        field_data_bit_start_index /*destination_bit_start_index*/,
        bit_count);

    field_data
}

#[test]
fn test_get_field_data() {
    // bit_count == 1, single byte
    assert_eq!(get_field_data::<1>(&[0b10110001], 0 /*bit_start_index*/, 1 /*bit_count*/), [0b00000001]);
    assert_eq!(get_field_data::<1>(&[0b10110001], 1 /*bit_start_index*/, 1 /*bit_count*/), [0b00000000]);
    assert_eq!(get_field_data::<1>(&[0b10110001], 2 /*bit_start_index*/, 1 /*bit_count*/), [0b00000001]);
    assert_eq!(get_field_data::<1>(&[0b10110001], 3 /*bit_start_index*/, 1 /*bit_count*/), [0b00000001]);
    assert_eq!(get_field_data::<1>(&[0b10110001], 4 /*bit_start_index*/, 1 /*bit_count*/), [0b00000000]);
    assert_eq!(get_field_data::<1>(&[0b10110001], 5 /*bit_start_index*/, 1 /*bit_count*/), [0b00000000]);
    assert_eq!(get_field_data::<1>(&[0b10110001], 6 /*bit_start_index*/, 1 /*bit_count*/), [0b00000000]);
    assert_eq!(get_field_data::<1>(&[0b10110001], 7 /*bit_start_index*/, 1 /*bit_count*/), [0b00000001]);

    // bit_count == 2, multiple bytes
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 0 /*bit_start_index*/, 2 /*bit_count*/), [0b00000010]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 1 /*bit_start_index*/, 2 /*bit_count*/), [0b00000001]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 2 /*bit_start_index*/, 2 /*bit_count*/), [0b00000011]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 3 /*bit_start_index*/, 2 /*bit_count*/), [0b00000010]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 4 /*bit_start_index*/, 2 /*bit_count*/), [0b00000000]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 5 /*bit_start_index*/, 2 /*bit_count*/), [0b00000000]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 6 /*bit_start_index*/, 2 /*bit_count*/), [0b00000001]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 7 /*bit_start_index*/, 2 /*bit_count*/), [0b00000011]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 8 /*bit_start_index*/, 2 /*bit_count*/), [0b00000011]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 9 /*bit_start_index*/, 2 /*bit_count*/), [0b00000011]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 10 /*bit_start_index*/, 2 /*bit_count*/), [0b00000010]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 11 /*bit_start_index*/, 2 /*bit_count*/), [0b00000000]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 12 /*bit_start_index*/, 2 /*bit_count*/), [0b00000001]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 13 /*bit_start_index*/, 2 /*bit_count*/), [0b00000010]);
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 14 /*bit_start_index*/, 2 /*bit_count*/), [0b00000001]);
    
    // bit_count == 6, multiple bytes
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 5 /*bit_start_index*/, 6 /*bit_count*/), [0b00001111]);

    // Data spanning multiple bytes, aligned on byte boundaries
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 8 /*bit_count*/), [0b10110001, 0, 0]);
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 16 /*bit_count*/), [0b10110001, 0b11100101, 0]);
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 24 /*bit_count*/), [0b10110001, 0b11100101, 0b00101110]);

    // Similar to previous, but not *quite* byte-boundary-aligned
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 1 /*bit_start_index*/, 23 /*bit_count*/), [0b00110001, 0b11100101, 0b00101110]);
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 23 /*bit_count*/), [0b01011000, 0b11110010, 0b10010111]);

    // Similar to previous, but using a larger field_data array than is needed
    assert_eq!(get_field_data::<5>(&[0b10110001, 0b11100101, 0b00101110], 1 /*bit_start_index*/, 23 /*bit_count*/), [0b00110001, 0b11100101, 0b00101110, 0, 0]);
    assert_eq!(get_field_data::<5>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 23 /*bit_count*/), [0b01011000, 0b11110010, 0b10010111, 0, 0]);

}