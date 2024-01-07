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
    for field_data_byte_index in 0 .. FIELD_DATA_BYTE_COUNT {
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
    assert!(bit_count <= FIELD_DATA_BYTE_COUNT * 8, "Unable to get a field value that is wider than {} bits.", FIELD_DATA_BYTE_COUNT * 8);

    let bit_end_index_exclusive = bit_start_index + bit_count;
    let bit_end_index_inclusive = bit_end_index_exclusive - 1;

    let byte_start_index = bit_start_index / 8;
    let byte_end_index_inclusive = bit_end_index_inclusive / 8;
    let byte_end_index_exclusive = byte_end_index_inclusive + 1;

    let bit_start_index_within_byte = bit_start_index % 8;
    let bit_end_index_exclusive_within_byte = (bit_end_index_inclusive % 8) + 1;

    let source_byte_count = byte_end_index_exclusive - byte_start_index;
    assert!(source_byte_count > 0);

    // We'll read from a slice of 'source_data' such that the Nth byte in 'source_data'
    // corresponds to the Nth byte in 'field_data'.  But for fields that span multiple
    // source bytes, the Nth byte of 'field_data' will also include some data from the (N+1)st
    // byte of 'source_data'.
    let field_source_data = &source_data[byte_start_index ..= byte_end_index_inclusive];

    let mut field_data: [u8; FIELD_DATA_BYTE_COUNT] = [0; FIELD_DATA_BYTE_COUNT];

    if (bit_start_index % 8) == 0 && (bit_end_index_exclusive % 8) == 0 {
        // The field data is aligned on byte boundaries, so we can use a simple approach
        // without masking or shifting.
        enumerate_bytes::<FIELD_DATA_BYTE_COUNT>(
            &mut field_data,
            source_byte_count,
            |field_data_byte_index: usize| -> u8 {
                field_source_data[field_data_byte_index]
            });
    } else if byte_start_index == byte_end_index_inclusive {
        // The field data is entirely contained within a single byte
        let mask = create_bit_mask(bit_start_index_within_byte, bit_count);
        let shift_right_bit_count = 8 - bit_end_index_exclusive_within_byte;

        enumerate_bytes::<FIELD_DATA_BYTE_COUNT>(
            &mut field_data,
            source_byte_count,
            |field_data_byte_index: usize| -> u8 {
                assert!(field_data_byte_index == 0);
                (field_source_data[field_data_byte_index] & mask) >> shift_right_bit_count
            });
    } else {
        // The field data is not contained in a single byte, nor is it neatly aligned to
        // byte boundaries.
        //
        // Use the most complicated approach, involving masking and shifting, to extract the
        // field data properly.
        //
        // We'll use little-endian byte ordering (least significant byte first), but within
        // each byte, the most significant bit is first and the least significant bit is last.
        // 
        // This means we want to return a byte array where the right-most bit in the field data
        // is the right-most bit of the last byte.
        // 
        // As an added twist, we want to handle the case where FIELD_DATA_BYTE_COUNT exceeds
        // the number of bytes in the actual field data.  In that case, we should put the remaining
        // unused bytes set to all zero at the *end* of the returned byte array.
        let actual_field_data_byte_count = (bit_count + 7) / 8;
        assert!(actual_field_data_byte_count > 0);

        let mut remaining_bit_count = bit_count;
        let mut source_byte_index = source_byte_count - 1;
        let mut returned_field_data_byte_index = actual_field_data_byte_count - 1;
        let mut consumed_bit_count_in_current_returned_field_data_byte = 0;
        let mut returned_field_data_byte: u8 = 0;

        loop {
            if remaining_bit_count == 0 {
                break;
            }

            if consumed_bit_count_in_current_returned_field_data_byte == 0 {
                // We haven't populated anything in the current returned_field_data_byte.

                let (mask, shift_right_bit_count, chunk_bit_count) =
                    if remaining_bit_count > bit_end_index_exclusive_within_byte {
                        // Extract the left part of the current source byte, and shift it right.
                        let chunk_bit_count = bit_end_index_exclusive_within_byte;

                        let mask = create_bit_mask(0, chunk_bit_count);
                        let shift_right_bit_count = 8 - chunk_bit_count;

                        (mask, shift_right_bit_count, chunk_bit_count)
                    } else {
                        // Extract what remains and shift it right.
                        let chunk_bit_count = remaining_bit_count;

                        let mask = create_bit_mask(bit_start_index_within_byte, chunk_bit_count);
                        let shift_right_bit_count = 8 - (bit_start_index_within_byte + chunk_bit_count);

                        (mask, shift_right_bit_count, chunk_bit_count)
                    };

                let extracted_data = (source_data[source_byte_index] & mask) >> shift_right_bit_count;
                returned_field_data_byte = returned_field_data_byte | extracted_data;

                consumed_bit_count_in_current_returned_field_data_byte = consumed_bit_count_in_current_returned_field_data_byte + chunk_bit_count;
                remaining_bit_count = remaining_bit_count - chunk_bit_count;

                // This source byte is complete now, but the returned_field_data_byte is still in progress.
                if source_byte_index == 0 {
                    // Done iterating.  Store the returned_field_data_byte that we've created so far.
                    field_data[returned_field_data_byte_index] = returned_field_data_byte;
                    break;
                }

                source_byte_index = source_byte_index - 1;
            } else {
                // We have already populated part of the current returned_field_data_byte.

                let (mask, shift_left_bit_count, chunk_bit_count) =
                    if bit_end_index_exclusive_within_byte == 8 {
                        // Special case where we know bit_count will be zero.
                        (0, 0, 0)
                    } else if remaining_bit_count >= (8 - bit_end_index_exclusive_within_byte) {
                        // Extract the right part of the current source byte, and shift it left.
                        let chunk_bit_count = 8 - bit_end_index_exclusive_within_byte;

                        let mask = create_bit_mask(bit_end_index_exclusive_within_byte, chunk_bit_count);
                        let shift_left_bit_count = bit_end_index_exclusive_within_byte;

                        (mask, shift_left_bit_count, chunk_bit_count)
                    } else {
                        // Extract what remains and shift it left.
                        let chunk_bit_count = remaining_bit_count;

                        let mask = create_bit_mask(bit_start_index_within_byte, chunk_bit_count);
                        let shift_left_bit_count = bit_end_index_exclusive_within_byte;

                        (mask, shift_left_bit_count, chunk_bit_count)
                    };

                let extracted_data = (source_data[source_byte_index] & mask) << shift_left_bit_count;
                returned_field_data_byte = returned_field_data_byte | extracted_data;

                consumed_bit_count_in_current_returned_field_data_byte = 0;
                remaining_bit_count = remaining_bit_count - chunk_bit_count;

                // This source byte is still in progress, but the returned_field_data_byte is complete.
                field_data[returned_field_data_byte_index] = returned_field_data_byte;
                returned_field_data_byte = 0;

                if returned_field_data_byte_index == 0 {
                    break;
                }
                
                returned_field_data_byte_index = returned_field_data_byte_index - 1;
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
    
    // bit_count == 6, multiple bytes
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 5 /*bit_start_index*/, 6 /*bit_count*/), [0b00001111]);

    // Data spanning multiple bytes, aligned on byte boundaries
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 8 /*bit_count*/), [0b10110001, 0, 0]);
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 16 /*bit_count*/), [0b10110001, 0b11100101, 0]);
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 24 /*bit_count*/), [0b10110001, 0b11100101, 0b00101110]);

    // Similar to previous, but not *quite* byte-boundary-aligned
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 1 /*bit_start_index*/, 23 /*bit_count*/), [0b00110001, 0b11100101, 0b00101110]);
    assert_eq!(get_field_data::<3>(&[0b10110001, 0b11100101, 0b00101110], 0 /*bit_start_index*/, 23 /*bit_count*/), [0b01011000, 0b11110010, 0b10010111]);
}