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
pub fn get_field_data<const FIELD_DATA_BYTE_COUNT: usize>(source_data: &[u8], bit_start_index: usize, bit_count: usize) -> [u8; FIELD_DATA_BYTE_COUNT] {
    assert!(bit_count > 0);
    assert!(bit_count < FIELD_DATA_BYTE_COUNT * 8, "Unable to get a field value that is wider than {} bits.", FIELD_DATA_BYTE_COUNT * 8);

    let bit_end_index_exclusive = bit_start_index + bit_count;

    let byte_start_index = bit_start_index / 8;
    let byte_end_index_exclusive = bit_end_index_exclusive / 8 + 1;

    assert!(byte_start_index < byte_end_index_exclusive);

    let byte_count = byte_end_index_exclusive - byte_start_index;

    let field_source_data = &source_data[byte_start_index .. byte_end_index_exclusive];

    let mut field_data: [u8; FIELD_DATA_BYTE_COUNT] = [0; FIELD_DATA_BYTE_COUNT];

    let bit_start_index_within_each_byte = bit_start_index % 8;

    if byte_count > 1 {
        let current_byte_shift_left_bit_count = bit_start_index_within_each_byte;
        let trailing_byte_shift_right_bit_count = 8 - bit_start_index_within_each_byte;

        let current_byte_mask_usize: usize = (1 << bit_start_index_within_each_byte) - 1;
        let trailing_byte_mask_usize: usize = 0xFF - current_byte_mask_usize;
        
        let current_byte_mask: u8 = current_byte_mask_usize.try_into().unwrap();
        let trailing_byte_mask: u8 = trailing_byte_mask_usize.try_into().unwrap();

        for (byte_index, source_data_byte) in field_source_data.iter().enumerate() {
            if byte_index >= FIELD_DATA_BYTE_COUNT {
                break;
            }
            
            let mut field_data_byte: u8 =
                if byte_index + 1 < byte_count {
                    (source_data_byte & current_byte_mask) << current_byte_shift_left_bit_count
                } else {
                    0
                };

            if byte_index + 1 < source_data.len() {
                // OR in the trailing part of the next byte
                let trailing_part_of_next_byte: u8 = source_data[byte_index + 1] & trailing_byte_mask;
                field_data_byte = field_data_byte | (trailing_part_of_next_byte >> trailing_byte_shift_right_bit_count);
            }

            field_data[byte_index] = field_data_byte;
        }
    } else /*byte_count == 1*/ {
        assert!(byte_count == 1);
    }

    field_data
}

#[test]
fn test() {
    assert_eq!(get_field_data::<1>(&[0b10110001, 0b11100101], 5 /*bit_start_index*/, 6 /*bit_count*/), [0b00111100]);

    //assert_eq!(get_field_data::<1>(&[0b10110001], 0 /*bit_start_index*/, 1 /*bit_count*/), [0b00000001]);
}