//! Functionality for handling ExtraBytes of Point records

use std::collections::HashMap;

use crate::header::Header;
use crate::point::Point;

/**
Nyi docs
*/
#[derive(Debug)]
pub struct ExtraBytesReader {
    vlrs: Vec<ExtraBytesVlr>,
    cumulative_byte_sizes: Vec<usize>,
    field_name_index: HashMap<String, usize>,
}

impl ExtraBytesReader {
    /// Nyi docs
    pub fn new(header: &Header) -> Self {
        let mut vlrs = vec![];
        for vlr in header.all_vlrs() {
            if vlr.user_id == "LASF_Spec" && vlr.record_id == 4 {
                eprintln!("Extra byte vlr lasf_spec present");

                // There should be a multiple of 192 bytes
                const SIZE: usize = std::mem::size_of::<ExtraBytesVlr>();
                let nr = vlr.data.len() / SIZE;

                for i in 0..nr {
                    // Use Cursor on vlr.data ?
                    let j = i * SIZE;
                    let bytes = ExtraBytesVlr {
                        _reserved: vlr.data[j..2 + j].try_into().unwrap(),
                        data_type: vlr.data[2 + j..3 + j].try_into().unwrap(),
                        options: vlr.data[3 + j..4 + j].try_into().unwrap(),
                        name: vlr.data[4 + j..36 + j].try_into().unwrap(),
                        _unused: vlr.data[36 + j..40 + j].try_into().unwrap(),
                        _no_data: vlr.data[40 + j..64 + j].try_into().unwrap(),
                        _min: vlr.data[64 + j..88 + j].try_into().unwrap(),
                        _max: vlr.data[88 + j..112 + j].try_into().unwrap(),
                        scale: vlr.data[112 + j..136 + j].try_into().unwrap(),
                        offset: vlr.data[136 + j..160 + j].try_into().unwrap(),
                        description: vlr.data[160 + j..192 + j].try_into().unwrap(),
                    };
                    vlrs.push(bytes);
                }
            }
        }

        // where do the extra fields start?
        let mut cumulative_byte_sizes = vec![];
        let mut acc = 0;
        for field in &vlrs {
            cumulative_byte_sizes.push(acc);
            acc += field.data_type().byte_size() as usize;
        }

        // at which index does a field (indexed by name) live
        let mut field_name_index = HashMap::new();
        for (index, field) in vlrs.iter().enumerate() {
            let name = field.name().to_owned();
            field_name_index.insert(name, index);
        }

        ExtraBytesReader {
            vlrs,
            cumulative_byte_sizes,
            field_name_index,
        }
    }

    /// Nyi docs
    pub fn has_extra_bytes(&self) -> bool {
        !self.vlrs.is_empty()
    }

    /// Nyi docs
    pub fn names(&self) -> Vec<String> {
        self.vlrs
            .iter()
            .map(|desc| desc.name().to_string())
            .collect()
    }

    /// Nyi docs
    pub fn descriptions(&self) -> Vec<String> {
        self.vlrs
            .iter()
            .map(|desc| desc.description().to_string())
            .collect()
    }

    /// Nyi docs
    pub fn all_values(&self, point: &Point) -> Vec<f64> {
        let mut result = vec![];
        self.cumulative_byte_sizes
            .iter()
            .zip(&self.vlrs)
            .for_each(|tup| {
                let start_index = *tup.0;
                let field = tup.1;
                let end_index = start_index + field.data_type().byte_size() as usize;
                let relevant_bytes = &point.extra_bytes[start_index..end_index];
                let value = match field.data_type() {
                    ExtraBytesDataType::Undocumented => 0.0,
                    ExtraBytesDataType::UnsignedChar => {
                        u8::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::Char => {
                        i8::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::UnsignedShort => {
                        u16::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::Short => {
                        i16::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::UnsignedLong => {
                        u32::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::Long => {
                        i32::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::UnsignedLongLong => {
                        u64::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::LongLong => {
                        i64::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::Float => {
                        f32::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
                    }
                    ExtraBytesDataType::Double => {
                        f64::from_le_bytes(relevant_bytes.try_into().unwrap())
                    }
                    ExtraBytesDataType::Nyi => 0.0,
                    ExtraBytesDataType::Reserved => 0.0,
                };
                let scale = field.get_scale();
                let offset = field.get_offset();
                result.push(value * scale + offset);
            });
        result
    }

    /// Nyi docs
    pub fn value_for_named_field(&self, name: &String, point: &Point) -> f64 {
        let index = self
            .field_name_index
            .get(name)
            .expect(format!("Unable to locate field with name '{name}'").as_str());
        let field = self.vlrs.get(*index).unwrap();
        let start_index = *self.cumulative_byte_sizes.get(*index).unwrap();
        let end_index = start_index + field.data_type().byte_size() as usize;
        let relevant_bytes = &point.extra_bytes[start_index..end_index];
        let value = match field.data_type() {
            ExtraBytesDataType::Undocumented => 0.0,
            ExtraBytesDataType::UnsignedChar => {
                u8::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::Char => i8::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64,
            ExtraBytesDataType::UnsignedShort => {
                u16::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::Short => {
                i16::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::UnsignedLong => {
                u32::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::Long => {
                i32::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::UnsignedLongLong => {
                u64::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::LongLong => {
                i64::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::Float => {
                f32::from_le_bytes(relevant_bytes.try_into().unwrap()) as f64
            }
            ExtraBytesDataType::Double => f64::from_le_bytes(relevant_bytes.try_into().unwrap()),
            ExtraBytesDataType::Nyi => 0.0,

            ExtraBytesDataType::Reserved => 0.0,
        };
        let scale = field.get_scale();
        let offset = field.get_offset();

        value * scale + offset
    }
}

/// "The Extra Bytes VLR provides a mechanism whereby additional information can be added to the end of a standard Point Record" page 24
#[derive(Debug)]
struct ExtraBytesVlr {
    _reserved: [u8; 2],
    data_type: [u8; 1],
    options: [u8; 1],
    name: [u8; 32],
    _unused: [u8; 4],
    // NYI: no_data / min / max
    _no_data: [u8; 24],
    _min: [u8; 24],
    _max: [u8; 24],
    scale: [u8; 24],
    offset: [u8; 24],
    description: [u8; 32],
}

#[derive(Debug)]
struct ExtraBytesOptions {
    options: u8,
}

/// Table 25, page 26
/// American Society for Photogrammetry & Remote Sensing Page 25 of 28
/// """
/// The bit mask in the options field specifies whether the min and max range of the value have been
/// set (i.e. are meaningful), whether the scale and/or offset values are set with which the “extra
/// bytes” are to be multiplied and translated to compute their actual value, and whether there is a
/// special value that should be interpreted as NO_DATA. By default all bits are zero which means
/// that the values in the corresponding fields are to be disregarded.
/// """
impl ExtraBytesOptions {
    /// 0 - no_data_bit - If set the no_data value is relevant
    fn _no_data_is_relevant(&self) -> bool {
        (self.options & 1) != 0
    }

    /// 1 - min_bit - If set the min value is relevant
    fn _min_value_is_relevant(&self) -> bool {
        (self.options & 2) != 0
    }

    /// 2 - max bit - If set the max value is relevant
    fn _max_value_is_relevant(&self) -> bool {
        (self.options & 4) != 0
    }

    /// 3 - scale_bit - If set each value should be multiplied by the corresponding scale value (before applying the offset)
    fn scale_bit_is_set(&self) -> bool {
        (self.options & 8) != 0
    }

    /// 4 - offset_bit - If set each value should be translated by the corresponding offset value (after applying the scaling).
    fn offset_bit_is_set(&self) -> bool {
        (self.options & 8) != 0
    }
}

impl ExtraBytesVlr {
    fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    fn options(&self) -> ExtraBytesOptions {
        ExtraBytesOptions {
            options: self.options[0],
        }
    }

    fn data_type(&self) -> ExtraBytesDataType {
        self.data_type[0].into()
    }

    fn description(&self) -> &str {
        std::str::from_utf8(&self.description)
            .unwrap()
            .trim_matches(char::from(0))
    }

    fn get_scale(&self) -> f64 {
        if self.options().scale_bit_is_set() {
            f64::from_le_bytes(self.scale[0..8].try_into().unwrap())
        } else {
            1.0
        }
    }

    fn get_offset(&self) -> f64 {
        if self.options().offset_bit_is_set() {
            f64::from_le_bytes(self.offset[0..8].try_into().unwrap())
        } else {
            0.0
        }
    }
}

// Table 24 of
// https://www.asprs.org/wp-content/uploads/2010/12/LAS_1_4_r13.pdf
// page 25
// 0 undocumented extra bytes specify value in options field
// 1 unsigned char      1 byte
// 2 char               1 byte
// 3 unsigned short     2 bytes
// 4 short              2 bytes
// 5 unsigned long      4 bytes
// 6 long               4 bytes
// 7 unsigned long long 8 bytes
// 8 long long          8 bytes
// 9 float              4 bytes
// 10 double            8 bytes
// 11-30 : 2d / 3d vectors (Not Yet Implemented)
// 31-255 Reserved / not assigned
#[derive(Debug)]
enum ExtraBytesDataType {
    Undocumented = 0,
    UnsignedChar,
    Char,
    UnsignedShort,
    Short,
    UnsignedLong,
    Long,
    UnsignedLongLong,
    LongLong,
    Float,
    Double,
    // We can have a fixed length vector as output :|
    // Maybe make an enum with the return value(s) embedded:
    // one, two, three values ?
    Nyi = 11,
    Reserved = 31,
}

impl From<u8> for ExtraBytesDataType {
    fn from(val: u8) -> Self {
        match val {
            0 => ExtraBytesDataType::Undocumented,
            1 => ExtraBytesDataType::UnsignedChar,
            2 => ExtraBytesDataType::Char,
            3 => ExtraBytesDataType::UnsignedShort,
            4 => ExtraBytesDataType::Short,
            5 => ExtraBytesDataType::UnsignedLong,
            6 => ExtraBytesDataType::Long,
            7 => ExtraBytesDataType::UnsignedLongLong,
            8 => ExtraBytesDataType::LongLong,
            9 => ExtraBytesDataType::Float,
            10 => ExtraBytesDataType::Double,
            11..=30 => ExtraBytesDataType::Nyi,
            31..=255 => ExtraBytesDataType::Reserved,
        }
    }
}

impl ExtraBytesDataType {
    fn byte_size(&self) -> u8 {
        match self {
            ExtraBytesDataType::Undocumented => 0,
            ExtraBytesDataType::UnsignedChar => 1,
            ExtraBytesDataType::Char => 1,
            ExtraBytesDataType::UnsignedShort => 2,
            ExtraBytesDataType::Short => 2,
            ExtraBytesDataType::UnsignedLong => 4,
            ExtraBytesDataType::Long => 4,
            ExtraBytesDataType::UnsignedLongLong => 8,
            ExtraBytesDataType::LongLong => 8,
            ExtraBytesDataType::Float => 4,
            ExtraBytesDataType::Double => 8,
            ExtraBytesDataType::Nyi => 0,
            ExtraBytesDataType::Reserved => 0,
        }
    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn test_size_of_extra_bytes_vlr() {
        assert_eq!(std::mem::size_of::<crate::point::extra_bytes::ExtraBytesVlr>(), 192);
    }
}