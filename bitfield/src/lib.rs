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

fn enumerate_bytes<const FIELD_DATA_BYTE_COUNT: usize>(
    field_data: &mut [u8; FIELD_DATA_BYTE_COUNT],
    source_byte_count: usize,
    callback: impl Fn(usize) -> u8) {
    for (field_data_byte_index, _) in [0 .. FIELD_DATA_BYTE_COUNT].iter().enumerate() {
        let field_data_byte =
            if field_data_byte_index >= source_byte_count {
                0
            } else {
                callback(field_data_byte_index)
            };

        field_data[field_data_byte_index] = field_data_byte;
    }
}

pub fn get_field_data<const FIELD_DATA_BYTE_COUNT: usize>(source_data: &[u8], bit_start_index: usize, bit_count: usize) -> [u8; FIELD_DATA_BYTE_COUNT] {
    assert!(bit_count > 0);
    assert!(bit_count < FIELD_DATA_BYTE_COUNT * 8, "Unable to get a field value that is wider than {} bits.", FIELD_DATA_BYTE_COUNT * 8);

    let bit_end_index_exclusive = bit_start_index + bit_count;
    let bit_end_index_inclusive = bit_end_index_exclusive - 1;

    let byte_start_index = bit_start_index / 8;
    let byte_end_index_inclusive = bit_end_index_inclusive / 8;
    let byte_end_index_exclusive = byte_end_index_inclusive + 1;

    let bit_start_index_within_byte = bit_start_index % 8;
    let bit_end_index_exclusive_within_byte = (bit_end_index_inclusive % 8) + 1;

    let source_byte_count = byte_end_index_exclusive - byte_start_index;

    // We'll read from a slice of 'source_data' such that the Nth byte in 'source_data'
    // corresponds to the Nth byte in 'field_data'.  But for fields that span multiple
    // source bytes, the Nth byte of 'field_data' will also include some data from the (N+1)st
    // byte of 'source_data'.
    let field_source_data = &source_data[byte_start_index ..= byte_end_index_inclusive];

    let mut field_data: [u8; FIELD_DATA_BYTE_COUNT] = [0; FIELD_DATA_BYTE_COUNT];

    if (bit_start_index % 8) == 0 && (bit_end_index_exclusive % 8) == 0 {
        // The field data is aligned on byte boundaries, so we can use a simple approach
        // without masking or shifting.
        enumerate_bytes(
            &mut field_data,
            source_byte_count,
            |field_data_byte_index: usize| -> u8 {
                field_source_data[field_data_byte_index]
            });
    } else if byte_start_index == byte_end_index_inclusive {
        // The field data is entirely contained within a single byte
        let mask = create_bit_mask(bit_start_index_within_byte, bit_count);
        let shift_right_bit_count = 8 - bit_end_index_exclusive_within_byte;

        enumerate_bytes(
            &mut field_data,
            source_byte_count,
            |field_data_byte_index: usize| -> u8 {
                assert!(field_data_byte_index == 0);
                (field_source_data[field_data_byte_index] & mask) >> shift_right_bit_count
            });
    } else {
        // The field data spans multiple bytes, and the start or end of the data are not
        // aligned on byte boundaries.
        // Use the most complicated approach with different masking and shifting for the first,
        // middle, and last bytes.
        //
        // Each source_data byte is divided into a left part and a right part.
        //
        // The bit_end_index determines most of our shifting and masking behavior.
        // We want to end up with data where the right-most bit of the field is aligned
        // with the right-most bit of the last byte of the resulting data.

        let middle_byte_left_part_bit_count = bit_end_index_exclusive_within_byte;
        let middle_byte_right_part_bit_count = 8 - bit_end_index_exclusive_within_byte;

        let middle_byte_left_part_mask = create_bit_mask(0, middle_byte_left_part_bit_count);
        let middle_byte_right_part_mask = create_bit_mask(middle_byte_left_part_bit_count, middle_byte_right_part_bit_count);

        // For bytes in the middle, the left part of each middle byte is treated as the right-most
        // (trailing) part of the resulting data byte, so it is shifted right by N bits.
        let middle_byte_left_part_shift_right_bit_count = 8 - middle_byte_left_part_bit_count;

        // For bytes in the middle, the right part of each middle byte is treated as the left-most
        // (leading) part of the resulting data byte, so it is shifted left by N bits.
        let middle_byte_right_part_shift_left_bit_count = 8 - middle_byte_right_part_bit_count;

        // The first byte is treated just like the right part of a middle byte, but with a different mask.
        let first_byte_right_part_bit_count = 8 - bit_start_index_within_byte;
        let first_byte_right_part_mask = create_bit_mask(bit_start_index_within_byte, first_byte_right_part_bit_count);

        let first_byte_right_part_shift_left_bit_count = middle_byte_right_part_shift_left_bit_count;

        enumerate_bytes(
            &mut field_data,
            source_byte_count,
            |field_data_byte_index: usize| -> u8 {
                let mut field_data_byte: u8 = 0;

                let current_source_byte = field_source_data[field_data_byte_index];

                // Extract the right portion of the current source byte and shift it left.
                let (mask, shift_left_bit_count) =
                    if field_data_byte_index == 0 {
                        // The first byte
                        (first_byte_right_part_mask, first_byte_right_part_shift_left_bit_count)
                    } else {
                        (middle_byte_right_part_mask, middle_byte_right_part_shift_left_bit_count)
                    };

                let extracted_data = (current_source_byte & mask) << shift_left_bit_count;
                field_data_byte = field_data_byte | extracted_data;

                if field_data_byte_index + 1 < source_byte_count {
                    // For all bytes but the last byte, extract the left portion of the next byte and shift it right.
                    let next_source_byte = field_source_data[field_data_byte_index + 1];

                    let extracted_data = (next_source_byte & middle_byte_left_part_mask) >> middle_byte_left_part_shift_right_bit_count;
                    field_data_byte = field_data_byte | extracted_data;
                }

                field_data_byte
            });
    }

    field_data
}

#[test]
fn test() {
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
    
    // byte_count > 1
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 5 /*bit_start_index*/, 6 /*bit_count*/), [0b00001111]);
}