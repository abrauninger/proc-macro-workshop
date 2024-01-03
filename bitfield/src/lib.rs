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

struct MaskAndShiftRight {
    mask: u8,
    shift_right_bit_count: u8,
}

struct MaskAndShiftLeft {
    mask: u8,
    shift_left_bit_count: u8,
}

struct MiddleByteMaskAndShift {
    left_part: MaskAndShiftRight,
    right_part: MaskAndShiftLeft,
}

struct MultiByteMaskAndShift {
    first_byte: MaskAndShiftLeft,
    middle_byte: MiddleByteMaskAndShift,
    last_byte: MaskAndShiftRight,
}

enum DataAccessMethod {
    NoMaskingOrShifting,
    MaskAndShiftSingleByte(MaskAndShiftRight),
    MaskAndShiftMultipleBytes(MultiByteMaskAndShift),
}

/// Returns a byte consisting of 1's in the bits [bit_start_index .. bit_start_index + bit_count],
/// and 0's otherwise.
/// Note that the leftmost bit is the 0th bit index.
fn create_bit_mask(bit_start_index: u8, bit_count: u8) -> u8 {
    assert!(bit_start_index < 8);
    assert!(bit_start_index + bit_count <= 8);

    let mask_usize: usize = ((1 << bit_count) - 1) << (8 - bit_start_index);
    mask_usize.try_into().unwrap()
}

fn get_data_access_method(bit_start_index: usize, bit_count: usize) -> DataAccessMethod {
    assert!(bit_count > 0);

    let bit_end_index_exclusive = bit_start_index + bit_count;
    let bit_end_index_inclusive = bit_end_index_exclusive - 1;

    let byte_start_index = bit_start_index / 8;
    let byte_end_index_inclusive = bit_end_index_inclusive / 8;

    if (bit_start_index % 8) == 0 && (bit_end_index_exclusive % 8) == 0 {
        DataAccessMethod::NoMaskingOrShifting
    } else if byte_start_index == byte_end_index_inclusive {
        let mask = create_bit_mask(bit_start_index, bit_count);
        let shift_right_bit_count = 8 - bit_end_index_exclusive;

        DataAccessMethod::SingleByte(MaskAndShiftRight { mask, shift_right_bit_count })
    } else {
        let bit_start_index_within_first_byte = bit_start_index % 8;
        let bit_end_index_exclusive_within_last_byte = bit_end_index_exclusive % 8;

        // Each source_data byte is divided into a left part and a right part.
        // The boundary between the left and right parts is determined by bit_start_index.
        let middle_byte_left_part_bit_count = bit_start_index_within_first_byte;
        let middle_byte_right_part_bit_count = 8 - bit_start_index_within_first_byte;

        // The bit_end_index determines most of our shifting and masking behavior.
        // We want to end up with data where the right-most bit of the field is aligned
        // with the right-most bit of the last byte of the resulting data.

        // For bytes in the middle, the right part of each middle byte is treated as the left-most
        // (leading) part of the resulting data byte, so it is shifted left by N bits.
        let middle_byte_right_part_shift_left_bit_count = bit_end_index_exclusive_within_last_byte;

        // For bytes in the middle, the left part of each middle byte is treated as the right-most
        // (trailing) part of the resulting data byte, so it is shifted right by N bits.
        let middle_byte_left_part_shift_right_bit_count = 8 - bit_end_index_exclusive_within_last_byte;

        let middle_byte_left_part_mask = create_bit_mask(, bit_count)


        let middle_byte_left_part_mask_usize = ((1 << middle_byte_left_part_bit_count) - 1) << middle_byte_right_part_shift_left_bit_count;
        let middle_byte_right_part_mask_usize = (1 << middle_byte_right_part_bit_count) - 1;

        let middle_byte_left_part_mask: u8 = middle_byte_left_part_mask_usize.try_into().unwrap();
        let middle_byte_right_part_mask: u8 = middle_byte_right_part_mask_usize.try_into().unwrap();

        // The first byte is treated just like the right part of a middle byte, but with a different mask.
        let first_byte_right_part_shift_left_bit_count = middle_byte_right_part_shift_left_bit_count;

        let first_byte_bit_count = 8 - bit_start_index_within_first_byte;
        let first_byte_right_part_mask_usize = (1 << first_byte_bit_count) - 1;
        let first_byte_right_part_mask: u8 = first_byte_right_part_mask_usize.try_into().unwrap();

        // The last byte is treated just like the left part of a middle byte, with the same mask and everything.
        let last_byte_left_part_shift_right_bit_count = middle_byte_right_part_shift_left_bit_count;
        let last_byte_left_part_mask = middle_byte_left_part_mask;
    };
}

// Helper functions
// TODO: Move these to a detail module?
pub fn get_field_data<const FIELD_DATA_BYTE_COUNT: usize>(source_data: &[u8], bit_start_index: usize, bit_count: usize) -> [u8; FIELD_DATA_BYTE_COUNT] {
    assert!(bit_count > 0);
    assert!(bit_count < FIELD_DATA_BYTE_COUNT * 8, "Unable to get a field value that is wider than {} bits.", FIELD_DATA_BYTE_COUNT * 8);

    let bit_end_index_exclusive = bit_start_index + bit_count;
    let bit_end_index_inclusive = bit_end_index_exclusive - 1;

    let byte_start_index = bit_start_index / 8;
    let byte_end_index_inclusive = bit_end_index_inclusive / 8;

    let source_byte_count = byte_end_index_exclusive - byte_start_index;

    // We'll read from a slice of 'source_data' such that the Nth byte in 'source_data'
    // corresponds to the Nth byte in 'field_data'.  But for fields that span multiple
    // source bytes, the Nth byte of 'field_data' will also include some data from the (N+1)st
    // byte of 'source_data'.
    let field_source_data = &source_data[byte_start_index ..= byte_end_index_inclusive];

    let mut field_data: [u8; FIELD_DATA_BYTE_COUNT] = [0; FIELD_DATA_BYTE_COUNT];

    match access_algorithm {
        DataAccessAlgorithm::SingleByte => {
            let mask_unshifted: usize = (1 << bit_count) - 1;
            let mask_usize: usize = mask_unshifted << (8 - bit_start_index - bit_count);
            let mask: u8 = mask_usize.try_into().unwrap();

            let masked_byte = source_data[0] & mask;
            let field_data_byte = masked_byte >> (8 - bit_start_index - bit_count);

            field_data[0] = field_data_byte;
        },
        _ => {
            let bit_start_index_within_first_byte = bit_start_index % 8;
            let bit_end_index_exclusive_within_last_byte = bit_end_index_exclusive % 8;

            // Each source_data byte is divided into a left part and a right part.
            // The boundary between the left and right parts is determined by bit_start_index.
            let middle_byte_left_part_bit_count = bit_start_index_within_first_byte;
            let middle_byte_right_part_bit_count = 8 - bit_start_index_within_first_byte;

            // The bit_end_index determines most of our shifting and masking behavior.
            // We want to end up with data where the right-most bit of the field is aligned
            // with the right-most bit of the last byte of the resulting data.

            // For bytes in the middle, the right part of each middle byte is treated as the left-most
            // (leading) part of the resulting data byte, so it is shifted left by N bits.
            let middle_byte_right_part_shift_left_bit_count = bit_end_index_exclusive_within_last_byte;

            // For bytes in the middle, the left part of each middle byte is treated as the right-most
            // (trailing) part of the resulting data byte, so it is shifted right by N bits.
            let middle_byte_left_part_shift_right_bit_count = 8 - bit_end_index_exclusive_within_last_byte;

            let middle_byte_left_part_mask_usize = ((1 << middle_byte_left_part_bit_count) - 1) << middle_byte_right_part_shift_left_bit_count;
            let middle_byte_right_part_mask_usize = (1 << middle_byte_right_part_bit_count) - 1;

            let middle_byte_left_part_mask: u8 = middle_byte_left_part_mask_usize.try_into().unwrap();
            let middle_byte_right_part_mask: u8 = middle_byte_right_part_mask_usize.try_into().unwrap();

            // The first byte is treated just like the right part of a middle byte, but with a different mask.
            let first_byte_right_part_shift_left_bit_count = middle_byte_right_part_shift_left_bit_count;

            let first_byte_bit_count = 8 - bit_start_index_within_first_byte;
            let first_byte_right_part_mask_usize = (1 << first_byte_bit_count) - 1;
            let first_byte_right_part_mask: u8 = first_byte_right_part_mask_usize.try_into().unwrap();

            // The last byte is treated just like the left part of a middle byte, with the same mask and everything.
            let last_byte_left_part_shift_right_bit_count = middle_byte_right_part_shift_left_bit_count;
            let last_byte_left_part_mask = middle_byte_left_part_mask;

            for (byte_index, _) in [0 .. source_byte_count].iter().enumerate() {
                let mut field_data_byte: u8 = 0;

                let current_source_byte = field_source_data[byte_index];

                if byte_index + 1 < byte_count - 1 {
                    // For all bytes but the last byte, extract the right portion of the byte and shift it left.
                    let (mask, shift_left_bit_count) =
                        if byte_index == 0 {
                            // The first byte
                            (first_byte_right_part_mask, first_byte_right_part_shift_left_bit_count)
                        } else {
                            (middle_byte_right_part_mask, middle_byte_right_part_shift_left_bit_count)
                        };

                    let extracted_data = (current_source_byte & mask) << shift_left_bit_count;
                    field_data_byte = field_data_byte | extracted_data;
                }

                if access_algorithm == DatAccessAlgorithm::byte_index > 0 {
                    // For all bytes but the first byte, extract the left portion of the byte and shift it right.
                    let (mask, shift_right_bit_count) =
                        if byte_index + 1 == byte_count - 1 {
                            // The last byte
                            (last_byte_left_part_mask, last_byte_left_part_shift_right_bit_count)
                        } else {
                            (middle_byte_left_part_mask, middle_byte_left_part_shift_right_bit_count)
                        };

                    let extracted_data = (current_source_byte & mask) >> shift_right_bit_count;
                    field_data_byte = field_data_byte | extracted_data;
                }

                field_data[byte_index] = field_data_byte;
            }
        }
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
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 5 /*bit_start_index*/, 6 /*bit_count*/), [0b00111100]);
}