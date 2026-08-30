//! Typed access to user-defined attributes stored after LAS point records.
//!
//! Extra Bytes are described by the `LASF_Spec` record 4 VLR. The parser
//! supports data types 0 through 10. Deprecated array types can be accessed as
//! raw bytes when reading, but typed writing rejects them; reserved types are
//! rejected because their layout is unknown.

use crate::{utils::FromLasStr, Builder, Error, Header, Point, PointData, Result, Vlr};
use std::{collections::HashSet, marker::PhantomData, ops::Range, slice::ChunksExact};

const NO_DATA_BIT: u8 = 1 << 0;
const MIN_BIT: u8 = 1 << 1;
const MAX_BIT: u8 = 1 << 2;
const SCALE_BIT: u8 = 1 << 3;
const OFFSET_BIT: u8 = 1 << 4;

/// The storage type declared by an Extra Bytes descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtraBytesDataType {
    /// Raw bytes; the descriptor's options byte is their on-disk width.
    Undocumented,
    /// An unsigned 8-bit integer.
    U8,
    /// A signed 8-bit integer.
    I8,
    /// An unsigned 16-bit integer.
    U16,
    /// A signed 16-bit integer.
    I16,
    /// An unsigned 32-bit integer.
    U32,
    /// A signed 32-bit integer.
    I32,
    /// An unsigned 64-bit integer.
    U64,
    /// A signed 64-bit integer.
    I64,
    /// A 32-bit IEEE floating-point number.
    F32,
    /// A 64-bit IEEE floating-point number.
    F64,
    /// A deprecated data type (codes 11 through 30).
    Deprecated(u8),
    /// A reserved data type (codes 31 through 255).
    Reserved(u8),
}

impl ExtraBytesDataType {
    /// Returns the numeric code stored in the descriptor.
    pub fn code(self) -> u8 {
        match self {
            Self::Undocumented => 0,
            Self::U8 => 1,
            Self::I8 => 2,
            Self::U16 => 3,
            Self::I16 => 4,
            Self::U32 => 5,
            Self::I32 => 6,
            Self::U64 => 7,
            Self::I64 => 8,
            Self::F32 => 9,
            Self::F64 => 10,
            Self::Deprecated(code) | Self::Reserved(code) => code,
        }
    }

    /// Returns true for one of the ten supported scalar numeric types.
    pub fn is_scalar(self) -> bool {
        matches!(
            self,
            Self::U8
                | Self::I8
                | Self::U16
                | Self::I16
                | Self::U32
                | Self::I32
                | Self::U64
                | Self::I64
                | Self::F32
                | Self::F64
        )
    }

    fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Undocumented,
            1 => Self::U8,
            2 => Self::I8,
            3 => Self::U16,
            4 => Self::I16,
            5 => Self::U32,
            6 => Self::I32,
            7 => Self::U64,
            8 => Self::I64,
            9 => Self::F32,
            10 => Self::F64,
            11..=30 => Self::Deprecated(code),
            31..=255 => Self::Reserved(code),
        }
    }

    fn scalar_size(self) -> Option<usize> {
        match self {
            Self::U8 | Self::I8 => Some(1),
            Self::U16 | Self::I16 => Some(2),
            Self::U32 | Self::I32 | Self::F32 => Some(4),
            Self::U64 | Self::I64 | Self::F64 => Some(8),
            Self::Undocumented | Self::Deprecated(_) | Self::Reserved(_) => None,
        }
    }

    fn byte_size(self, options: u8) -> Option<usize> {
        self.scalar_size().or_else(|| match self {
            Self::Undocumented => Some(usize::from(options)),
            Self::Deprecated(code @ 11..=20) => {
                Self::from_code(code - 10).scalar_size().map(|bs| bs * 2)
            }
            Self::Deprecated(code @ 21..=30) => {
                Self::from_code(code - 20).scalar_size().map(|bs| bs * 3)
            }
            Self::Reserved(_) => None,
            _ => None,
        })
    }
}

/// A numeric Extra Bytes value.
///
/// Unscaled unsigned integers are represented as `Unsigned`, unscaled signed
/// integers as `Signed`, and floating-point or transformed values as `Float`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExtraBytesValue {
    /// An unsigned 64-bit integer.
    Unsigned(u64),
    /// A signed 64-bit integer.
    Signed(i64),
    /// A 64-bit IEEE floating-point number.
    Float(f64),
}

impl ExtraBytesValue {
    /// Converts this value to `f64`.
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Unsigned(value) => value as f64,
            Self::Signed(value) => value as f64,
            Self::Float(value) => value,
        }
    }
}

macro_rules! impl_extra_bytes_value_from {
    ($variant:ident, $($type:ty),+ $(,)?) => {
        $(
            impl From<$type> for ExtraBytesValue {
                fn from(value: $type) -> Self {
                    Self::$variant(value.into())
                }
            }
        )+
    };
}

impl_extra_bytes_value_from!(Unsigned, u8, u16, u32, u64);
impl_extra_bytes_value_from!(Signed, i8, i16, i32, i64);
impl_extra_bytes_value_from!(Float, f32, f64);

/// Metadata for one attribute in an Extra Bytes VLR.
#[derive(Clone, Debug)]
pub struct ExtraBytesDescriptor {
    data_type: ExtraBytesDataType,
    options: u8,
    name: String,
    no_data: Option<ExtraBytesValue>,
    min: Option<ExtraBytesValue>,
    max: Option<ExtraBytesValue>,
    scale: f64,
    offset: f64,
    description: String,
    byte_offset: usize,
    byte_size: usize,
}

impl ExtraBytesDescriptor {
    /// Creates a scalar numeric Extra Bytes descriptor for writing.
    ///
    /// Use [`ExtraBytesDescriptor::undocumented`] for a raw byte field.
    pub fn new(name: impl Into<String>, data_type: ExtraBytesDataType) -> Result<Self> {
        let name = name.into();
        validate_fixed_string(&name)?;
        let byte_size = data_type
            .scalar_size()
            .ok_or_else(|| Error::NonNumericExtraBytesField(name.clone()))?;
        Ok(Self {
            data_type,
            options: 0,
            name,
            no_data: None,
            min: None,
            max: None,
            scale: 1.0,
            offset: 0.0,
            description: String::new(),
            byte_offset: 0,
            byte_size,
        })
    }

    /// Creates an undocumented raw-byte descriptor for writing.
    pub fn undocumented(name: impl Into<String>, byte_size: u8) -> Result<Self> {
        let name = name.into();
        validate_fixed_string(&name)?;
        Ok(Self {
            data_type: ExtraBytesDataType::Undocumented,
            options: byte_size,
            name,
            no_data: None,
            min: None,
            max: None,
            scale: 1.0,
            offset: 0.0,
            description: String::new(),
            byte_offset: 0,
            byte_size: usize::from(byte_size),
        })
    }

    /// Sets this descriptor's human-readable description.
    pub fn with_description(mut self, description: impl Into<String>) -> Result<Self> {
        let description = description.into();
        validate_fixed_string(&description)?;
        self.description = description;
        Ok(self)
    }

    /// Sets the raw no-data value and enables the descriptor's no-data bit.
    pub fn with_no_data(mut self, value: impl Into<ExtraBytesValue>) -> Result<Self> {
        let value = value.into();
        validate_metadata_value(self.data_type, value, &self.name)?;
        self.no_data = Some(value);
        self.options |= NO_DATA_BIT;
        Ok(self)
    }

    /// Sets the raw minimum value and enables the descriptor's minimum bit.
    pub fn with_min(mut self, value: impl Into<ExtraBytesValue>) -> Result<Self> {
        let value = value.into();
        validate_metadata_value(self.data_type, value, &self.name)?;
        self.min = Some(value);
        self.options |= MIN_BIT;
        Ok(self)
    }

    /// Sets the raw maximum value and enables the descriptor's maximum bit.
    pub fn with_max(mut self, value: impl Into<ExtraBytesValue>) -> Result<Self> {
        let value = value.into();
        validate_metadata_value(self.data_type, value, &self.name)?;
        self.max = Some(value);
        self.options |= MAX_BIT;
        Ok(self)
    }

    /// Sets a non-zero finite scale and enables the descriptor's scale bit.
    pub fn with_scale(mut self, scale: f64) -> Result<Self> {
        if !scale.is_finite() || scale == 0.0 {
            return Err(Error::InvalidExtraBytesValue(self.name.clone()));
        }
        self.scale = scale;
        self.options |= SCALE_BIT;
        Ok(self)
    }

    /// Sets a finite offset and enables the descriptor's offset bit.
    pub fn with_offset(mut self, offset: f64) -> Result<Self> {
        if !offset.is_finite() {
            return Err(Error::InvalidExtraBytesValue(self.name.clone()));
        }
        self.offset = offset;
        self.options |= OFFSET_BIT;
        Ok(self)
    }

    /// Returns the attribute's unique name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the attribute's description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the attribute's storage type.
    pub fn data_type(&self) -> ExtraBytesDataType {
        self.data_type
    }

    /// Returns the raw options byte.
    pub fn options(&self) -> u8 {
        self.options
    }

    /// Returns the offset within a point's Extra Bytes region.
    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Returns the number of bytes occupied in each point record.
    pub fn byte_size(&self) -> usize {
        self.byte_size
    }

    /// Returns the exact no-data value when its option bit is set.
    pub fn no_data(&self) -> Option<ExtraBytesValue> {
        self.no_data
    }

    /// Returns the exact minimum value when its option bit is set.
    pub fn min(&self) -> Option<ExtraBytesValue> {
        self.min
    }

    /// Returns the exact maximum value when its option bit is set.
    pub fn max(&self) -> Option<ExtraBytesValue> {
        self.max
    }

    /// Returns the scale, or one when its option bit is clear.
    pub fn scale(&self) -> f64 {
        if self.data_type.is_scalar() && self.options & SCALE_BIT != 0 {
            self.scale
        } else {
            1.0
        }
    }

    /// Returns the offset, or zero when its option bit is clear.
    pub fn offset(&self) -> f64 {
        if self.data_type.is_scalar() && self.options & OFFSET_BIT != 0 {
            self.offset
        } else {
            0.0
        }
    }

    fn decode(&self, bytes: &[u8]) -> Option<ExtraBytesValue> {
        let value = decode_value(self.data_type, bytes)?;
        if self.options & (SCALE_BIT | OFFSET_BIT) != 0 {
            Some(ExtraBytesValue::Float(
                value.as_f64() * self.scale() + self.offset(),
            ))
        } else {
            Some(value)
        }
    }

    fn is_no_data(&self, value: ExtraBytesValue) -> bool {
        self.no_data == Some(value)
    }

    fn range(&self) -> Range<usize> {
        self.byte_offset..self.byte_offset + self.byte_size
    }

    fn encode_into(&self, value: ExtraBytesValue, bytes: &mut [u8]) -> Result<()> {
        let value = self.to_storage_value(value)?;
        encode_storage_value(self.data_type, value, &self.name, bytes)
    }

    fn encode_no_data_into(&self, bytes: &mut [u8]) -> Result<()> {
        let value = self
            .no_data
            .ok_or_else(|| Error::ExtraBytesNoDataNotDefined(self.name.clone()))?;
        encode_storage_value(self.data_type, value, &self.name, bytes)
    }

    fn to_storage_value(&self, value: ExtraBytesValue) -> Result<ExtraBytesValue> {
        if self.options & (SCALE_BIT | OFFSET_BIT) == 0 {
            return Ok(value);
        }
        let ExtraBytesValue::Float(value) = value else {
            return Err(Error::InvalidExtraBytesValue(self.name.clone()));
        };
        let scale = self.scale();
        let offset = self.offset();
        if !value.is_finite() || !scale.is_finite() || scale == 0.0 || !offset.is_finite() {
            return Err(Error::InvalidExtraBytesValue(self.name.clone()));
        }
        let raw = (value - offset) / scale;
        match self.data_type {
            ExtraBytesDataType::U8
            | ExtraBytesDataType::U16
            | ExtraBytesDataType::U32
            | ExtraBytesDataType::U64 => float_to_unsigned(raw, &self.name),
            ExtraBytesDataType::I8
            | ExtraBytesDataType::I16
            | ExtraBytesDataType::I32
            | ExtraBytesDataType::I64 => float_to_signed(raw, &self.name),
            ExtraBytesDataType::F32 | ExtraBytesDataType::F64 => Ok(ExtraBytesValue::Float(raw)),
            ExtraBytesDataType::Undocumented
            | ExtraBytesDataType::Deprecated(_)
            | ExtraBytesDataType::Reserved(_) => {
                Err(Error::NonNumericExtraBytesField(self.name.clone()))
            }
        }
    }
}

/// Parsed Extra Bytes metadata and typed accessors for points and byte slabs.
#[derive(Clone, Debug)]
pub struct ExtraBytesVlr {
    descriptors: Vec<ExtraBytesDescriptor>,
    total_bytes_len: usize,
}

impl TryFrom<&Vlr> for ExtraBytesVlr {
    type Error = Error;

    fn try_from(value: &Vlr) -> Result<Self> {
        if !value.is_extra_bytes() {
            return Err(Error::NotExtraBytesVlr);
        }
        Self::from_payload(value.data.as_slice())
    }
}

impl TryFrom<&ExtraBytesVlr> for Vlr {
    type Error = Error;

    fn try_from(value: &ExtraBytesVlr) -> Result<Self> {
        let payload_len = value
            .descriptors
            .len()
            .checked_mul(ExtraBytesVlr::DESCRIPTOR_SIZE)
            .ok_or(Error::VlrTooLong(usize::MAX))?;
        if payload_len > u16::MAX as usize {
            return Err(Error::VlrTooLong(payload_len));
        }
        let mut data = Vec::with_capacity(payload_len);
        for descriptor in &value.descriptors {
            data.extend_from_slice(&encode_descriptor(descriptor)?);
        }
        Ok(Vlr {
            user_id: ExtraBytesVlr::USER_ID.to_owned(),
            record_id: ExtraBytesVlr::RECORD_ID,
            description: ExtraBytesVlr::DESCRIPTION.to_owned(),
            data,
        })
    }
}

impl TryFrom<ExtraBytesVlr> for Vlr {
    type Error = Error;

    fn try_from(value: ExtraBytesVlr) -> Result<Self> {
        Self::try_from(&value)
    }
}

impl Vlr {
    /// Returns true if this is the LASF_Spec Extra Bytes VLR.
    pub fn is_extra_bytes(&self) -> bool {
        self.user_id == ExtraBytesVlr::USER_ID && self.record_id == ExtraBytesVlr::RECORD_ID
    }
}

impl ExtraBytesVlr {
    /// The registered user ID of the Extra Bytes VLR.
    pub const USER_ID: &'static str = "LASF_Spec";
    /// The record ID of the Extra Bytes VLR.
    pub const RECORD_ID: u16 = 4;
    /// The description of the Extra Bytes VLR.
    pub const DESCRIPTION: &'static str = "Extra Bytes Record";
    /// The encoded size of one Extra Bytes descriptor.
    pub const DESCRIPTOR_SIZE: usize = 192;

    /// Parses and validates Extra Bytes descriptors from a header.
    ///
    /// This compatibility constructor is equivalent to [`Header::extra_bytes_vlr`].
    pub fn new(header: &Header) -> Result<Option<Self>> {
        header.extra_bytes_vlr()
    }

    /// Creates an Extra Bytes schema for writing from descriptors in on-disk order.
    pub fn from_descriptors<I>(descriptors: I) -> Result<Self>
    where
        I: IntoIterator<Item = ExtraBytesDescriptor>,
    {
        let mut descriptors: Vec<_> = descriptors.into_iter().collect();
        let mut names = HashSet::with_capacity(descriptors.len());
        let mut described_bytes_len = 0usize;
        for descriptor in &mut descriptors {
            if matches!(
                descriptor.data_type,
                ExtraBytesDataType::Deprecated(_) | ExtraBytesDataType::Reserved(_)
            ) {
                return Err(Error::UnsupportedExtraBytesDataType(
                    descriptor.data_type.code(),
                ));
            }
            validate_fixed_string(&descriptor.name)?;
            validate_fixed_string(&descriptor.description)?;
            if !names.insert(descriptor.name.clone()) {
                return Err(Error::DuplicateExtraBytesField(descriptor.name.clone()));
            }
            descriptor.byte_size = descriptor.data_type.byte_size(descriptor.options).ok_or(
                Error::ReservedExtraBytesDataType(descriptor.data_type.code()),
            )?;
            descriptor.byte_offset = described_bytes_len;
            described_bytes_len = described_bytes_len
                .checked_add(descriptor.byte_size)
                .ok_or(Error::VlrTooLong(usize::MAX))?;
        }
        let _ = u16::try_from(described_bytes_len)?;
        let payload_len = descriptors
            .len()
            .checked_mul(Self::DESCRIPTOR_SIZE)
            .ok_or(Error::VlrTooLong(usize::MAX))?;
        if payload_len > u16::MAX as usize {
            return Err(Error::VlrTooLong(payload_len));
        }
        Ok(Self {
            descriptors,
            total_bytes_len: described_bytes_len,
        })
    }

    /// Sets the number of undescribed trailing bytes in a schema for writing.
    pub fn with_trailing_bytes(mut self, byte_size: u16) -> Result<Self> {
        self.total_bytes_len = self
            .described_bytes_len()
            .checked_add(usize::from(byte_size))
            .ok_or(Error::VlrTooLong(usize::MAX))?;
        let _ = u16::try_from(self.total_bytes_len)?;
        Ok(self)
    }

    fn from_header(header: &Header) -> Result<Option<Self>> {
        let matching: Vec<_> = header
            .all_vlrs()
            .filter(|vlr| vlr.is_extra_bytes())
            .collect();

        if matching.is_empty() {
            return Ok(None);
        } else if matching.len() > 1 {
            return Err(Error::MultipleExtraBytesVlrs(matching.len()));
        }

        let mut extra_bytes = Self::from_payload(matching[0].data.as_slice())?;
        let actual_extra_len = usize::from(header.point_format().extra_bytes);
        let described_bytes_len = extra_bytes.described_bytes_len();
        if described_bytes_len > actual_extra_len {
            return Err(Error::ExtraBytesMismatch(
                described_bytes_len,
                actual_extra_len,
            ));
        }
        extra_bytes.total_bytes_len = actual_extra_len;
        Ok(Some(extra_bytes))
    }

    fn from_payload(data: &[u8]) -> Result<Self> {
        if !data.len().is_multiple_of(Self::DESCRIPTOR_SIZE) {
            return Err(Error::InvalidExtraBytesVlrLength(data.len()));
        }
        let mut descriptors = Vec::with_capacity(data.len() / Self::DESCRIPTOR_SIZE);
        let mut names = HashSet::new();
        let mut described_bytes_len = 0;
        for bytes in data.as_chunks::<{ Self::DESCRIPTOR_SIZE }>().0 {
            let descriptor = parse_descriptor(bytes, described_bytes_len)?;
            if !names.insert(descriptor.name.clone()) {
                return Err(Error::DuplicateExtraBytesField(descriptor.name));
            }
            described_bytes_len = described_bytes_len
                .checked_add(descriptor.byte_size)
                .ok_or(Error::VlrTooLong(usize::MAX))?;
            descriptors.push(descriptor);
        }
        Ok(Self {
            descriptors,
            total_bytes_len: described_bytes_len,
        })
    }

    /// Returns true when the point format contains trailing Extra Bytes.
    pub fn has_extra_bytes(&self) -> bool {
        self.total_bytes_len != 0
    }

    /// Returns true when at least one descriptor was parsed.
    pub fn has_descriptors(&self) -> bool {
        !self.descriptors.is_empty()
    }

    /// Returns all descriptors in on-disk order.
    pub fn descriptors(&self) -> &[ExtraBytesDescriptor] {
        &self.descriptors
    }

    /// Returns a descriptor by name.
    pub fn descriptor(&self, name: &str) -> Option<&ExtraBytesDescriptor> {
        self.descriptors.iter().find(|d| d.name() == name)
    }

    /// Returns descriptor names in on-disk order.
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.descriptors.iter().map(|descriptor| &descriptor.name)
    }

    /// Returns descriptor descriptions in on-disk order.
    pub fn descriptions(&self) -> impl Iterator<Item = &String> {
        self.descriptors
            .iter()
            .map(|descriptor| &descriptor.description)
    }

    /// Returns the total physical Extra Bytes width per point.
    pub fn total_bytes_len(&self) -> usize {
        self.total_bytes_len
    }

    /// Returns the bytes covered by descriptors.
    pub fn described_bytes_len(&self) -> usize {
        self.descriptors.last().map_or(0, |descriptor| {
            descriptor.byte_offset + descriptor.byte_size
        })
    }

    /// Returns the undescribed trailing bytes per point.
    pub fn undocumented_bytes_len(&self) -> usize {
        self.total_bytes_len - self.described_bytes_len()
    }

    /// Returns a zero-filled Extra Bytes region suitable for a new point.
    pub fn zeroed_point_bytes(&self) -> Vec<u8> {
        vec![0; self.total_bytes_len]
    }

    /// Resets a point's Extra Bytes region to this schema's width.
    pub fn initialize_point(&self, point: &mut Point) {
        point.extra_bytes = self.zeroed_point_bytes();
    }
}

impl Header {
    /// Returns this header's parsed Extra Bytes VLR, if present.
    ///
    /// The descriptor widths are validated against the point record's actual
    /// Extra Bytes width. Undescribed trailing bytes remain available through
    /// raw point and column accessors.
    pub fn extra_bytes_vlr(&self) -> Result<Option<ExtraBytesVlr>> {
        ExtraBytesVlr::from_header(self)
    }
}

impl Builder {
    /// Installs an Extra Bytes VLR and synchronizes the point-record width.
    ///
    /// Any existing Extra Bytes VLR, regular or extended, is replaced. The
    /// typed record is written as a regular VLR.
    pub fn set_extra_bytes_vlr(&mut self, extra_bytes: &ExtraBytesVlr) -> Result<()> {
        let total_bytes_len = u16::try_from(extra_bytes.total_bytes_len)?;
        let vlr = Vlr::try_from(extra_bytes)?;
        self.vlrs.retain(|vlr| !vlr.is_extra_bytes());
        self.evlrs.retain(|vlr| !vlr.is_extra_bytes());
        self.point_format.extra_bytes = total_bytes_len;
        self.vlrs.push(vlr);
        Ok(())
    }
}

impl PointData {
    /// Returns a named extra-bytes numeric column.
    ///
    /// Integer values are widened to `U64` or `I64`. Floating-point values and
    /// values with scale or offset enabled are returned as `F64`.
    pub fn extra_column<'pointdata, 'vlr>(
        &'pointdata self,
        descriptor: &'vlr ExtraBytesDescriptor,
    ) -> Result<Option<ExtraBytesColumn<'pointdata, 'vlr>>> {
        if !descriptor.data_type.is_scalar() {
            return Err(Error::NonNumericExtraBytesField(descriptor.name.clone()));
        }
        let offset = self.extra_field_offset(descriptor)?;
        let records = self.raw_bytes().chunks_exact(self.record_len());
        if descriptor.options & (SCALE_BIT | OFFSET_BIT) != 0 {
            Ok(Some(ExtraBytesColumn::Float(ExtraBytesTypedColumn {
                records,
                record_field_offset: offset,
                descriptor,
                item: PhantomData::<f64>,
            })))
        } else {
            match descriptor.data_type {
                ExtraBytesDataType::U8
                | ExtraBytesDataType::U16
                | ExtraBytesDataType::U32
                | ExtraBytesDataType::U64 => {
                    Ok(Some(ExtraBytesColumn::Unsigned(ExtraBytesTypedColumn {
                        records,
                        record_field_offset: offset,
                        descriptor,
                        item: PhantomData::<u64>,
                    })))
                }
                ExtraBytesDataType::I8
                | ExtraBytesDataType::I16
                | ExtraBytesDataType::I32
                | ExtraBytesDataType::I64 => {
                    Ok(Some(ExtraBytesColumn::Signed(ExtraBytesTypedColumn {
                        records,
                        record_field_offset: offset,
                        descriptor,
                        item: PhantomData::<i64>,
                    })))
                }
                ExtraBytesDataType::F32 | ExtraBytesDataType::F64 => {
                    Ok(Some(ExtraBytesColumn::Float(ExtraBytesTypedColumn {
                        records,
                        record_field_offset: offset,
                        descriptor,
                        item: PhantomData::<f64>,
                    })))
                }
                ExtraBytesDataType::Undocumented
                | ExtraBytesDataType::Deprecated(_)
                | ExtraBytesDataType::Reserved(_) => {
                    Err(Error::NonNumericExtraBytesField(descriptor.name.clone()))
                }
            }
        }
    }

    /// Returns a typed nullable column, mapping raw no-data values to `None`.
    pub fn extra_column_nullable<'pointdata, 'vlr>(
        &'pointdata self,
        descriptor: &'vlr ExtraBytesDescriptor,
    ) -> Result<Option<ExtraBytesNullableColumn<'pointdata, 'vlr>>> {
        if !descriptor.data_type.is_scalar() {
            return Err(Error::NonNumericExtraBytesField(descriptor.name.to_owned()));
        }
        let offset = self.extra_field_offset(descriptor)?;
        let records = self.raw_bytes().chunks_exact(self.record_len());
        if descriptor.options & (SCALE_BIT | OFFSET_BIT) != 0 {
            Ok(Some(ExtraBytesNullableColumn::Float(
                ExtraBytesTypedNullableColumn {
                    records,
                    record_field_offset: offset,
                    descriptor,
                    item: PhantomData,
                },
            )))
        } else {
            match descriptor.data_type {
                ExtraBytesDataType::U8
                | ExtraBytesDataType::U16
                | ExtraBytesDataType::U32
                | ExtraBytesDataType::U64 => Ok(Some(ExtraBytesNullableColumn::Unsigned(
                    ExtraBytesTypedNullableColumn {
                        records,
                        record_field_offset: offset,
                        descriptor,
                        item: PhantomData,
                    },
                ))),
                ExtraBytesDataType::I8
                | ExtraBytesDataType::I16
                | ExtraBytesDataType::I32
                | ExtraBytesDataType::I64 => Ok(Some(ExtraBytesNullableColumn::Signed(
                    ExtraBytesTypedNullableColumn {
                        records,
                        record_field_offset: offset,
                        descriptor,
                        item: PhantomData,
                    },
                ))),
                ExtraBytesDataType::F32 | ExtraBytesDataType::F64 => Ok(Some(
                    ExtraBytesNullableColumn::Float(ExtraBytesTypedNullableColumn {
                        records,
                        record_field_offset: offset,
                        descriptor,
                        item: PhantomData,
                    }),
                )),
                ExtraBytesDataType::Undocumented
                | ExtraBytesDataType::Deprecated(_)
                | ExtraBytesDataType::Reserved(_) => {
                    Err(Error::NonNumericExtraBytesField(descriptor.name.to_owned()))
                }
            }
        }
    }

    /// Returns the raw bytes for a named descriptor.
    pub fn extra_column_raw<'pointdata>(
        &'pointdata self,
        descriptor: &ExtraBytesDescriptor,
    ) -> Result<Option<ExtraBytesRawColumn<'pointdata>>> {
        let offset = self.extra_field_offset(descriptor)?;
        Ok(Some(ExtraBytesRawColumn {
            records: self.raw_bytes().chunks_exact(self.record_len()),
            record_field_offset: offset,
            byte_size: descriptor.byte_size,
        }))
    }

    /// Returns undescribed trailing bytes for every point.
    pub fn extra_column_undocumented<'pointdata>(
        &'pointdata self,
        vlr: &ExtraBytesVlr,
    ) -> Option<ExtraBytesRawColumn<'pointdata>> {
        if usize::from(self.format().extra_bytes) != vlr.total_bytes_len {
            return None;
        }
        let size = vlr.undocumented_bytes_len();
        if size == 0 {
            return None;
        }
        let offset =
            self.record_len() - self.format().extra_bytes as usize + vlr.described_bytes_len();
        Some(ExtraBytesRawColumn {
            records: self.raw_bytes().chunks_exact(self.record_len()),
            record_field_offset: offset,
            byte_size: size,
        })
    }

    /// Encodes a complete typed Extra Bytes column.
    ///
    /// The number of values must equal the number of points. Encoding is
    /// validated before the destination slab is modified.
    pub fn set_extra_column<I, V>(
        &mut self,
        descriptor: &ExtraBytesDescriptor,
        values: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = V>,
        V: Into<ExtraBytesValue>,
    {
        let offset = self.extra_field_offset(descriptor)?;
        let mut encoded = Vec::new();
        let mut value_count = 0;
        for value in values {
            let start = encoded.len();
            encoded.resize(start + descriptor.byte_size, 0);
            descriptor.encode_into(value.into(), &mut encoded[start..])?;
            value_count += 1;
        }
        self.set_encoded_column(offset, descriptor.byte_size, value_count, encoded)
    }

    /// Encodes a complete nullable typed Extra Bytes column.
    ///
    /// `None` is encoded using the descriptor's raw no-data value.
    pub fn set_extra_column_nullable<I, V>(
        &mut self,
        descriptor: &ExtraBytesDescriptor,
        values: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = Option<V>>,
        V: Into<ExtraBytesValue>,
    {
        let offset = self.extra_field_offset(descriptor)?;
        let mut encoded = Vec::new();
        let mut value_count = 0;
        for value in values {
            let start = encoded.len();
            encoded.resize(start + descriptor.byte_size, 0);
            match value {
                Some(value) => {
                    descriptor.encode_into(value.into(), &mut encoded[start..])?;
                }
                None => descriptor.encode_no_data_into(&mut encoded[start..])?,
            }
            value_count += 1;
        }
        self.set_encoded_column(offset, descriptor.byte_size, value_count, encoded)
    }

    /// Writes a complete raw Extra Bytes column.
    pub fn set_extra_column_raw<I, B>(
        &mut self,
        descriptor: &ExtraBytesDescriptor,
        values: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let offset = self.extra_field_offset(descriptor)?;
        let mut encoded = Vec::new();
        let mut value_count = 0;
        for value in values {
            let value = value.as_ref();
            if value.len() != descriptor.byte_size {
                return Err(Error::ExtraBytesFieldLengthMismatch {
                    name: descriptor.name.clone(),
                    expected: descriptor.byte_size,
                    actual: value.len(),
                });
            }
            encoded.extend_from_slice(value);
            value_count += 1;
        }
        self.set_encoded_column(offset, descriptor.byte_size, value_count, encoded)
    }

    fn extra_field_offset(&self, descriptor: &ExtraBytesDescriptor) -> Result<usize> {
        let actual = usize::from(self.format().extra_bytes);
        let required = descriptor.range().end;
        if required > actual {
            return Err(Error::PointDataExtraBytesMismatch(required, actual));
        }
        Ok(self.record_len() - actual + descriptor.byte_offset)
    }

    fn set_encoded_column(
        &mut self,
        offset: usize,
        byte_size: usize,
        value_count: usize,
        encoded: Vec<u8>,
    ) -> Result<()> {
        let expected = self.len();
        if value_count != expected {
            return Err(Error::ExtraBytesColumnLengthMismatch {
                expected,
                actual: value_count,
            });
        }
        if byte_size == 0 {
            return Ok(());
        }
        let record_len = self.record_len();
        for (record, value) in self
            .resize_for(expected)
            .chunks_exact_mut(record_len)
            .zip(encoded.chunks_exact(byte_size))
        {
            record[offset..offset + byte_size].copy_from_slice(value);
        }
        Ok(())
    }
}

impl Point {
    /// Returns one numeric Extra Bytes value from an owned point.
    pub fn extra_field(&self, descriptor: &ExtraBytesDescriptor) -> Result<ExtraBytesValue> {
        let range = descriptor.range();
        if range.end > self.extra_bytes.len() {
            return Err(Error::PointExtraBytesMismatch(
                range.end,
                self.extra_bytes.len(),
            ));
        }
        descriptor
            .decode(&self.extra_bytes[range])
            .ok_or_else(|| Error::NonNumericExtraBytesField(descriptor.name.to_owned()))
    }

    /// Returns the raw bytes for a named field from a borrowed point.
    ///
    /// The returned slice has the descriptor's on-disk width. It is not
    /// decoded, and scale and offset are not applied.
    pub fn extra_field_raw<'point>(
        &'point self,
        descriptor: &ExtraBytesDescriptor,
    ) -> Result<&'point [u8]> {
        let range = descriptor.range();
        if range.end > self.extra_bytes.len() {
            return Err(Error::PointExtraBytesMismatch(
                range.end,
                self.extra_bytes.len(),
            ));
        }
        Ok(&self.extra_bytes[range])
    }

    /// Encodes one typed Extra Bytes value into this point.
    pub fn set_extra_field(
        &mut self,
        descriptor: &ExtraBytesDescriptor,
        value: impl Into<ExtraBytesValue>,
    ) -> Result<()> {
        let range = descriptor.range();
        if range.end > self.extra_bytes.len() {
            return Err(Error::PointExtraBytesMismatch(
                range.end,
                self.extra_bytes.len(),
            ));
        }
        descriptor.encode_into(value.into(), &mut self.extra_bytes[range])?;
        Ok(())
    }

    /// Encodes an optional typed Extra Bytes value into this point.
    ///
    /// `None` is encoded using the descriptor's raw no-data value.
    pub fn set_extra_field_nullable<V>(
        &mut self,
        descriptor: &ExtraBytesDescriptor,
        value: Option<V>,
    ) -> Result<()>
    where
        V: Into<ExtraBytesValue>,
    {
        let range = descriptor.range();
        if range.end > self.extra_bytes.len() {
            return Err(Error::PointExtraBytesMismatch(
                range.end,
                self.extra_bytes.len(),
            ));
        }
        match value {
            Some(value) => descriptor.encode_into(value.into(), &mut self.extra_bytes[range])?,
            None => descriptor.encode_no_data_into(&mut self.extra_bytes[range])?,
        }
        Ok(())
    }

    /// Writes one raw Extra Bytes value into this point.
    pub fn set_extra_field_raw(
        &mut self,
        descriptor: &ExtraBytesDescriptor,
        value: &[u8],
    ) -> Result<()> {
        if value.len() != descriptor.byte_size {
            return Err(Error::ExtraBytesFieldLengthMismatch {
                name: descriptor.name.clone(),
                expected: descriptor.byte_size,
                actual: value.len(),
            });
        }
        let range = descriptor.range();
        if range.end > self.extra_bytes.len() {
            return Err(Error::PointExtraBytesMismatch(
                range.end,
                self.extra_bytes.len(),
            ));
        }
        self.extra_bytes[range].copy_from_slice(value);
        Ok(())
    }
}

/// A numeric Extra Bytes column whose variant determines every item's type.
#[derive(Clone, Debug)]
pub enum ExtraBytesColumn<'pointdata, 'vlr> {
    /// An iterator over unsigned integer values.
    Unsigned(ExtraBytesTypedColumn<'pointdata, 'vlr, u64>),
    /// An iterator over signed integer values.
    Signed(ExtraBytesTypedColumn<'pointdata, 'vlr, i64>),
    /// An iterator over floating-point values, including transformed values.
    Float(ExtraBytesTypedColumn<'pointdata, 'vlr, f64>),
}

impl ExtraBytesColumn<'_, '_> {
    /// Returns the number of values remaining in this column.
    pub fn len(&self) -> usize {
        match self {
            Self::Unsigned(values) => values.len(),
            Self::Signed(values) => values.len(),
            Self::Float(values) => values.len(),
        }
    }

    /// Returns true when this column contains no remaining values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An iterator over one concrete numeric Extra Bytes type.
#[derive(Clone, Debug)]
pub struct ExtraBytesTypedColumn<'pointdata, 'vlr, T> {
    records: ChunksExact<'pointdata, u8>,
    record_field_offset: usize,
    descriptor: &'vlr ExtraBytesDescriptor,
    item: PhantomData<T>,
}

impl Iterator for ExtraBytesTypedColumn<'_, '_, u64> {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        self.records.next().map(|record| {
            let bytes = &record
                [self.record_field_offset..self.record_field_offset + self.descriptor.byte_size];
            match decode_value(self.descriptor.data_type, bytes) {
                Some(ExtraBytesValue::Unsigned(value)) => value,
                _ => unreachable!("validated unsigned Extra Bytes column"),
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.records.size_hint()
    }
}
impl ExactSizeIterator for ExtraBytesTypedColumn<'_, '_, u64> {}

impl Iterator for ExtraBytesTypedColumn<'_, '_, i64> {
    type Item = i64;
    fn next(&mut self) -> Option<Self::Item> {
        self.records.next().map(|record| {
            let bytes = &record
                [self.record_field_offset..self.record_field_offset + self.descriptor.byte_size];
            match decode_value(self.descriptor.data_type, bytes) {
                Some(ExtraBytesValue::Signed(value)) => value,
                _ => unreachable!("validated signed Extra Bytes column"),
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.records.size_hint()
    }
}
impl ExactSizeIterator for ExtraBytesTypedColumn<'_, '_, i64> {}

impl Iterator for ExtraBytesTypedColumn<'_, '_, f64> {
    type Item = f64;
    fn next(&mut self) -> Option<Self::Item> {
        self.records.next().map(|record| {
            let bytes = &record
                [self.record_field_offset..self.record_field_offset + self.descriptor.byte_size];
            match self.descriptor.decode(bytes) {
                Some(ExtraBytesValue::Float(value)) => value,
                _ => unreachable!("validated floating-point Extra Bytes column"),
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.records.size_hint()
    }
}
impl ExactSizeIterator for ExtraBytesTypedColumn<'_, '_, f64> {}

/// A nullable Extra Bytes column whose variant determines every item's type.
#[derive(Clone, Debug)]
pub enum ExtraBytesNullableColumn<'pointdata, 'vlr> {
    /// An iterator over optional unsigned integer values.
    Unsigned(ExtraBytesTypedNullableColumn<'pointdata, 'vlr, u64>),
    /// An iterator over optional signed integer values.
    Signed(ExtraBytesTypedNullableColumn<'pointdata, 'vlr, i64>),
    /// An iterator over optional floating-point or transformed values.
    Float(ExtraBytesTypedNullableColumn<'pointdata, 'vlr, f64>),
}

impl ExtraBytesNullableColumn<'_, '_> {
    /// Returns the number of values remaining in this column.
    pub fn len(&self) -> usize {
        match self {
            Self::Unsigned(values) => values.len(),
            Self::Signed(values) => values.len(),
            Self::Float(values) => values.len(),
        }
    }

    /// Returns true when this column contains no remaining values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An iterator over one concrete optional numeric Extra Bytes type.
#[derive(Clone, Debug)]
pub struct ExtraBytesTypedNullableColumn<'pointdata, 'vlr, T> {
    records: ChunksExact<'pointdata, u8>,
    record_field_offset: usize,
    descriptor: &'vlr ExtraBytesDescriptor,
    item: PhantomData<T>,
}

impl<T> ExtraBytesTypedNullableColumn<'_, '_, T> {
    fn next_value(&mut self) -> Option<Option<ExtraBytesValue>> {
        self.records.next().map(|record| {
            let bytes = &record
                [self.record_field_offset..self.record_field_offset + self.descriptor.byte_size];
            let raw_value = decode_value(self.descriptor.data_type, bytes)
                .expect("validated Extra Bytes column");
            if self.descriptor.is_no_data(raw_value) {
                None
            } else {
                Some(
                    self.descriptor
                        .decode(bytes)
                        .expect("validated Extra Bytes column"),
                )
            }
        })
    }
}

impl Iterator for ExtraBytesTypedNullableColumn<'_, '_, u64> {
    type Item = Option<u64>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_value().map(|value| {
            value.map(|value| match value {
                ExtraBytesValue::Unsigned(value) => value,
                _ => unreachable!("validated unsigned Extra Bytes column"),
            })
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.records.size_hint()
    }
}
impl ExactSizeIterator for ExtraBytesTypedNullableColumn<'_, '_, u64> {}

impl Iterator for ExtraBytesTypedNullableColumn<'_, '_, i64> {
    type Item = Option<i64>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_value().map(|value| {
            value.map(|value| match value {
                ExtraBytesValue::Signed(value) => value,
                _ => unreachable!("validated signed Extra Bytes column"),
            })
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.records.size_hint()
    }
}
impl ExactSizeIterator for ExtraBytesTypedNullableColumn<'_, '_, i64> {}

impl Iterator for ExtraBytesTypedNullableColumn<'_, '_, f64> {
    type Item = Option<f64>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_value().map(|value| {
            value.map(|value| match value {
                ExtraBytesValue::Float(value) => value,
                _ => unreachable!("validated floating-point Extra Bytes column"),
            })
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.records.size_hint()
    }
}
impl ExactSizeIterator for ExtraBytesTypedNullableColumn<'_, '_, f64> {}

/// A raw byte column borrowed from a byte slab.
#[derive(Clone, Debug)]
pub struct ExtraBytesRawColumn<'a> {
    records: ChunksExact<'a, u8>,
    record_field_offset: usize,
    byte_size: usize,
}
impl<'a> Iterator for ExtraBytesRawColumn<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        self.records.next().map(|record| {
            &record[self.record_field_offset..self.record_field_offset + self.byte_size]
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.records.size_hint()
    }
}
impl ExactSizeIterator for ExtraBytesRawColumn<'_> {}

fn validate_fixed_string(value: &str) -> Result<()> {
    if !value.is_ascii() {
        return Err(Error::NotAscii(value.to_owned()));
    }
    let mut bytes = [0; 32];
    bytes.as_mut().from_las_str(value)
}

fn validate_metadata_value(
    data_type: ExtraBytesDataType,
    value: ExtraBytesValue,
    name: &str,
) -> Result<()> {
    let byte_size = data_type
        .scalar_size()
        .ok_or_else(|| Error::NonNumericExtraBytesField(name.to_owned()))?;
    let mut bytes = [0; 8];
    encode_storage_value(data_type, value, name, &mut bytes[..byte_size])
}

fn encode_storage_value(
    data_type: ExtraBytesDataType,
    value: ExtraBytesValue,
    name: &str,
    bytes: &mut [u8],
) -> Result<()> {
    match (data_type, value) {
        (ExtraBytesDataType::U8, ExtraBytesValue::Unsigned(value)) => {
            bytes
                .copy_from_slice(&[u8::try_from(value)
                    .map_err(|_| Error::InvalidExtraBytesValue(name.to_owned()))?]);
        }
        (ExtraBytesDataType::I8, ExtraBytesValue::Signed(value)) => bytes.copy_from_slice(
            &i8::try_from(value)
                .map_err(|_| Error::InvalidExtraBytesValue(name.to_owned()))?
                .to_le_bytes(),
        ),
        (ExtraBytesDataType::U16, ExtraBytesValue::Unsigned(value)) => bytes.copy_from_slice(
            &u16::try_from(value)
                .map_err(|_| Error::InvalidExtraBytesValue(name.to_owned()))?
                .to_le_bytes(),
        ),
        (ExtraBytesDataType::I16, ExtraBytesValue::Signed(value)) => bytes.copy_from_slice(
            &i16::try_from(value)
                .map_err(|_| Error::InvalidExtraBytesValue(name.to_owned()))?
                .to_le_bytes(),
        ),
        (ExtraBytesDataType::U32, ExtraBytesValue::Unsigned(value)) => bytes.copy_from_slice(
            &u32::try_from(value)
                .map_err(|_| Error::InvalidExtraBytesValue(name.to_owned()))?
                .to_le_bytes(),
        ),
        (ExtraBytesDataType::I32, ExtraBytesValue::Signed(value)) => bytes.copy_from_slice(
            &i32::try_from(value)
                .map_err(|_| Error::InvalidExtraBytesValue(name.to_owned()))?
                .to_le_bytes(),
        ),
        (ExtraBytesDataType::U64, ExtraBytesValue::Unsigned(value)) => {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
        (ExtraBytesDataType::I64, ExtraBytesValue::Signed(value)) => {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
        (ExtraBytesDataType::F32, ExtraBytesValue::Float(value)) => {
            let narrowed = value as f32;
            if value.is_finite() && !narrowed.is_finite() {
                return Err(Error::InvalidExtraBytesValue(name.to_owned()));
            } else {
                bytes.copy_from_slice(&narrowed.to_le_bytes());
            }
        }
        (ExtraBytesDataType::F64, ExtraBytesValue::Float(value)) => {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
        (
            ExtraBytesDataType::Undocumented
            | ExtraBytesDataType::Deprecated(_)
            | ExtraBytesDataType::Reserved(_),
            _,
        ) => return Err(Error::NonNumericExtraBytesField(name.to_owned())),
        _ => return Err(Error::InvalidExtraBytesValue(name.to_owned())),
    }
    Ok(())
}

fn float_to_unsigned(value: f64, name: &str) -> Result<ExtraBytesValue> {
    const U64_UPPER_EXCLUSIVE: f64 = 18_446_744_073_709_551_616.0;
    let rounded = rounded_float(value, name)?;
    if !(0.0..U64_UPPER_EXCLUSIVE).contains(&rounded) {
        Err(Error::InvalidExtraBytesValue(name.to_owned()))
    } else {
        Ok(ExtraBytesValue::Unsigned(rounded as u64))
    }
}

fn float_to_signed(value: f64, name: &str) -> Result<ExtraBytesValue> {
    const I64_LOWER_INCLUSIVE: f64 = -9_223_372_036_854_775_808.0;
    const I64_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
    let rounded = rounded_float(value, name)?;
    if !(I64_LOWER_INCLUSIVE..I64_UPPER_EXCLUSIVE).contains(&rounded) {
        Err(Error::InvalidExtraBytesValue(name.to_owned()))
    } else {
        Ok(ExtraBytesValue::Signed(rounded as i64))
    }
}

fn rounded_float(value: f64, name: &str) -> Result<f64> {
    if !value.is_finite() {
        return Err(Error::InvalidExtraBytesValue(name.to_owned()));
    }
    let rounded = value.round();
    let tolerance = f64::EPSILON * value.abs().max(1.0) * 8.0;
    if (value - rounded).abs() > tolerance {
        Err(Error::InvalidExtraBytesValue(name.to_owned()))
    } else {
        Ok(rounded)
    }
}

fn encode_metadata_value(
    data_type: ExtraBytesDataType,
    value: ExtraBytesValue,
    name: &str,
) -> Result<[u8; 8]> {
    validate_metadata_value(data_type, value, name)?;
    Ok(match value {
        ExtraBytesValue::Unsigned(value) => value.to_le_bytes(),
        ExtraBytesValue::Signed(value) => value.to_le_bytes(),
        ExtraBytesValue::Float(value) => value.to_le_bytes(),
    })
}

fn encode_descriptor(descriptor: &ExtraBytesDescriptor) -> Result<[u8; 192]> {
    if matches!(
        descriptor.data_type,
        ExtraBytesDataType::Deprecated(_) | ExtraBytesDataType::Reserved(_)
    ) {
        return Err(Error::UnsupportedExtraBytesDataType(
            descriptor.data_type.code(),
        ));
    }
    let mut bytes = [0; ExtraBytesVlr::DESCRIPTOR_SIZE];
    bytes[2] = descriptor.data_type.code();
    bytes[3] = descriptor.options;
    bytes[4..36].as_mut().from_las_str(&descriptor.name)?;
    if let Some(value) = descriptor.no_data {
        bytes[40..48].copy_from_slice(&encode_metadata_value(
            descriptor.data_type,
            value,
            &descriptor.name,
        )?);
    }
    if let Some(value) = descriptor.min {
        bytes[64..72].copy_from_slice(&encode_metadata_value(
            descriptor.data_type,
            value,
            &descriptor.name,
        )?);
    }
    if let Some(value) = descriptor.max {
        bytes[88..96].copy_from_slice(&encode_metadata_value(
            descriptor.data_type,
            value,
            &descriptor.name,
        )?);
    }
    if descriptor.options & SCALE_BIT != 0 {
        bytes[112..120].copy_from_slice(&descriptor.scale.to_le_bytes());
    }
    if descriptor.options & OFFSET_BIT != 0 {
        bytes[136..144].copy_from_slice(&descriptor.offset.to_le_bytes());
    }
    bytes[160..192]
        .as_mut()
        .from_las_str(&descriptor.description)?;
    Ok(bytes)
}

fn parse_descriptor(bytes: &[u8], byte_offset: usize) -> Result<ExtraBytesDescriptor> {
    if bytes.len() != ExtraBytesVlr::DESCRIPTOR_SIZE {
        return Err(Error::InvalidExtraBytesVlrLength(bytes.len()));
    }
    let data_type = ExtraBytesDataType::from_code(bytes[2]);
    let options = bytes[3];
    let byte_size = data_type
        .byte_size(options)
        .ok_or(Error::ReservedExtraBytesDataType(data_type.code()))?;
    Ok(ExtraBytesDescriptor {
        data_type,
        options,
        name: fixed_string(&bytes[4..36]),
        no_data: decode_metadata(
            data_type,
            options,
            NO_DATA_BIT,
            bytes[40..48].try_into().expect("descriptor field"),
        ),
        min: decode_metadata(
            data_type,
            options,
            MIN_BIT,
            bytes[64..72].try_into().expect("descriptor field"),
        ),
        max: decode_metadata(
            data_type,
            options,
            MAX_BIT,
            bytes[88..96].try_into().expect("descriptor field"),
        ),
        scale: f64::from_le_bytes(bytes[112..120].try_into().expect("descriptor field")),
        offset: f64::from_le_bytes(bytes[136..144].try_into().expect("descriptor field")),
        description: fixed_string(&bytes[160..192]),
        byte_offset,
        byte_size,
    })
}

fn decode_metadata(
    data_type: ExtraBytesDataType,
    options: u8,
    option_bit: u8,
    bytes: [u8; 8],
) -> Option<ExtraBytesValue> {
    if options & option_bit == 0 || !data_type.is_scalar() {
        return None;
    }
    Some(match data_type {
        ExtraBytesDataType::U8
        | ExtraBytesDataType::U16
        | ExtraBytesDataType::U32
        | ExtraBytesDataType::U64 => ExtraBytesValue::Unsigned(u64::from_le_bytes(bytes)),
        ExtraBytesDataType::I8
        | ExtraBytesDataType::I16
        | ExtraBytesDataType::I32
        | ExtraBytesDataType::I64 => ExtraBytesValue::Signed(i64::from_le_bytes(bytes)),
        ExtraBytesDataType::F32 | ExtraBytesDataType::F64 => {
            ExtraBytesValue::Float(f64::from_le_bytes(bytes))
        }
        ExtraBytesDataType::Undocumented
        | ExtraBytesDataType::Deprecated(_)
        | ExtraBytesDataType::Reserved(_) => {
            unreachable!("unsupported Extra Bytes type")
        }
    })
}

fn decode_value(data_type: ExtraBytesDataType, bytes: &[u8]) -> Option<ExtraBytesValue> {
    Some(match data_type {
        ExtraBytesDataType::U8 => {
            ExtraBytesValue::Unsigned(u64::from(u8::from_le_bytes(bytes.try_into().ok()?)))
        }
        ExtraBytesDataType::I8 => {
            ExtraBytesValue::Signed(i64::from(i8::from_le_bytes(bytes.try_into().ok()?)))
        }
        ExtraBytesDataType::U16 => {
            ExtraBytesValue::Unsigned(u64::from(u16::from_le_bytes(bytes.try_into().ok()?)))
        }
        ExtraBytesDataType::I16 => {
            ExtraBytesValue::Signed(i64::from(i16::from_le_bytes(bytes.try_into().ok()?)))
        }
        ExtraBytesDataType::U32 => {
            ExtraBytesValue::Unsigned(u64::from(u32::from_le_bytes(bytes.try_into().ok()?)))
        }
        ExtraBytesDataType::I32 => {
            ExtraBytesValue::Signed(i64::from(i32::from_le_bytes(bytes.try_into().ok()?)))
        }
        ExtraBytesDataType::U64 => {
            ExtraBytesValue::Unsigned(u64::from_le_bytes(bytes.try_into().ok()?))
        }
        ExtraBytesDataType::I64 => {
            ExtraBytesValue::Signed(i64::from_le_bytes(bytes.try_into().ok()?))
        }
        ExtraBytesDataType::F32 => {
            ExtraBytesValue::Float(f64::from(f32::from_le_bytes(bytes.try_into().ok()?)))
        }
        ExtraBytesDataType::F64 => {
            ExtraBytesValue::Float(f64::from_le_bytes(bytes.try_into().ok()?))
        }
        ExtraBytesDataType::Undocumented
        | ExtraBytesDataType::Deprecated(_)
        | ExtraBytesDataType::Reserved(_) => return None,
    })
}

fn fixed_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{point::Format, PointDataBuilder, Reader, Writer};
    use std::io::Cursor;

    fn descriptor_with_metadata(
        data_type: u8,
        options: u8,
        no_data: [u8; 8],
        min: [u8; 8],
        max: [u8; 8],
    ) -> ExtraBytesDescriptor {
        let mut bytes = [0; ExtraBytesVlr::DESCRIPTOR_SIZE];
        bytes[2] = data_type;
        bytes[3] = options;
        bytes[40..48].copy_from_slice(&no_data);
        bytes[64..72].copy_from_slice(&min);
        bytes[88..96].copy_from_slice(&max);
        parse_descriptor(&bytes, 0).expect("valid descriptor")
    }

    fn point_data_with_u16(values: &[u16]) -> PointData {
        let mut format = Format::new(0).expect("point format");
        format.extra_bytes = 2;
        let record_len = usize::from(format.len());
        let mut bytes = vec![0; record_len * values.len()];
        for (record, value) in bytes.chunks_exact_mut(record_len).zip(values) {
            record[record_len - 2..].copy_from_slice(&value.to_le_bytes());
        }
        PointDataBuilder::new()
            .with_format(format)
            .build_from_bytes(bytes)
            .expect("point data")
    }

    #[test]
    fn unsigned_metadata_stays_upcast() {
        let descriptor = descriptor_with_metadata(
            ExtraBytesDataType::U8.code(),
            NO_DATA_BIT | MIN_BIT | MAX_BIT,
            255_u64.to_le_bytes(),
            1_u64.to_le_bytes(),
            250_u64.to_le_bytes(),
        );
        assert_eq!(descriptor.no_data(), Some(ExtraBytesValue::Unsigned(255)));
        assert_eq!(descriptor.min(), Some(ExtraBytesValue::Unsigned(1)));
        assert_eq!(descriptor.max(), Some(ExtraBytesValue::Unsigned(250)));
    }

    #[test]
    fn signed_metadata_stays_upcast() {
        let descriptor = descriptor_with_metadata(
            ExtraBytesDataType::I16.code(),
            NO_DATA_BIT | MIN_BIT | MAX_BIT,
            (-32_768_i64).to_le_bytes(),
            (-12_345_i64).to_le_bytes(),
            12_345_i64.to_le_bytes(),
        );
        assert_eq!(descriptor.no_data(), Some(ExtraBytesValue::Signed(-32_768)));
        assert_eq!(descriptor.min(), Some(ExtraBytesValue::Signed(-12_345)));
        assert_eq!(descriptor.max(), Some(ExtraBytesValue::Signed(12_345)));
    }

    #[test]
    fn f32_metadata_keeps_stored_f64_precision() {
        let no_data = 1.234_567_890_123_f64;
        let descriptor = descriptor_with_metadata(
            ExtraBytesDataType::F32.code(),
            NO_DATA_BIT,
            no_data.to_le_bytes(),
            [0; 8],
            [0; 8],
        );
        assert_eq!(descriptor.no_data(), Some(ExtraBytesValue::Float(no_data)));
        assert_eq!(descriptor.min(), None);
        assert_eq!(descriptor.max(), None);
    }

    #[test]
    fn point_values_are_widened_to_three_public_types() {
        assert_eq!(
            decode_value(ExtraBytesDataType::U16, &42_u16.to_le_bytes()),
            Some(ExtraBytesValue::Unsigned(42))
        );
        assert_eq!(
            decode_value(ExtraBytesDataType::I8, &(-12_i8).to_le_bytes()),
            Some(ExtraBytesValue::Signed(-12))
        );
        assert_eq!(
            decode_value(ExtraBytesDataType::F32, &1.25_f32.to_le_bytes()),
            Some(ExtraBytesValue::Float(1.25))
        );
    }

    #[test]
    fn scale_or_offset_makes_the_value_f64() {
        let mut descriptor = descriptor_with_metadata(
            ExtraBytesDataType::U16.code(),
            SCALE_BIT | OFFSET_BIT,
            [0; 8],
            [0; 8],
            [0; 8],
        );
        descriptor.scale = 0.5;
        descriptor.offset = 10.0;
        assert_eq!(
            descriptor.decode(&20_u16.to_le_bytes()),
            Some(ExtraBytesValue::Float(20.0))
        );
    }

    #[test]
    fn column_selects_one_primitive_iterator_type() {
        let descriptor =
            descriptor_with_metadata(ExtraBytesDataType::U16.code(), 0, [0; 8], [0; 8], [0; 8]);

        let points = point_data_with_u16(&[12, 34]);

        match points
            .extra_column(&descriptor)
            .expect("column")
            .expect("field exists")
        {
            ExtraBytesColumn::Unsigned(values) => {
                assert_eq!(values.collect::<Vec<_>>(), vec![12_u64, 34]);
            }
            ExtraBytesColumn::Signed(_) | ExtraBytesColumn::Float(_) => {
                panic!("expected unsigned column")
            }
        }
    }

    #[test]
    fn transformed_integer_column_selects_float_iterator() {
        let mut descriptor = descriptor_with_metadata(
            ExtraBytesDataType::U16.code(),
            SCALE_BIT | OFFSET_BIT,
            [0; 8],
            [0; 8],
            [0; 8],
        );
        descriptor.scale = 0.5;
        descriptor.offset = 10.0;
        let points = point_data_with_u16(&[20, 40]);

        match points
            .extra_column(&descriptor)
            .expect("column")
            .expect("field exists")
        {
            ExtraBytesColumn::Float(values) => {
                assert_eq!(values.collect::<Vec<_>>(), vec![20.0_f64, 30.0]);
            }
            ExtraBytesColumn::Unsigned(_) | ExtraBytesColumn::Signed(_) => {
                panic!("expected float column")
            }
        }
    }

    #[test]
    fn nullable_column_also_selects_one_primitive_iterator_type() {
        let descriptor = descriptor_with_metadata(
            ExtraBytesDataType::U16.code(),
            NO_DATA_BIT,
            0_u64.to_le_bytes(),
            [0; 8],
            [0; 8],
        );
        let points = point_data_with_u16(&[0, 7]);

        match points
            .extra_column_nullable(&descriptor)
            .expect("nullable column")
            .expect("field exists")
        {
            ExtraBytesNullableColumn::Unsigned(values) => {
                assert_eq!(values.collect::<Vec<_>>(), vec![None, Some(7_u64)]);
            }
            ExtraBytesNullableColumn::Signed(_) | ExtraBytesNullableColumn::Float(_) => {
                panic!("expected unsigned column")
            }
        }
    }

    #[test]
    fn raw_named_value_borrows_the_point_bytes() {
        let mut descriptor =
            descriptor_with_metadata(ExtraBytesDataType::U16.code(), 0, [0; 8], [0; 8], [0; 8]);
        descriptor.name = "temperature".to_owned();
        let point = Point {
            extra_bytes: vec![0x34, 0x12],
            ..Point::default()
        };

        let raw = point.extra_field_raw(&descriptor).expect("named raw value");
        assert_eq!(raw, &[0x34, 0x12]);
        assert_eq!(raw.as_ptr(), point.extra_bytes.as_ptr());
    }

    #[test]
    fn header_width_controls_undocumented_trailing_bytes() {
        let descriptor =
            ExtraBytesDescriptor::new("value", ExtraBytesDataType::U16).expect("descriptor");
        let extra_bytes = ExtraBytesVlr::from_descriptors([descriptor]).expect("schema");
        let raw_vlr = Vlr::try_from(&extra_bytes).expect("VLR");

        let mut builder = Builder::default();
        builder.point_format.extra_bytes = 4;
        builder.vlrs.push(raw_vlr);
        let header = builder.into_header().expect("header");
        let parsed = header
            .extra_bytes_vlr()
            .expect("valid VLR")
            .expect("Extra Bytes VLR");

        assert_eq!(parsed.described_bytes_len(), 2);
        assert_eq!(parsed.total_bytes_len(), 4);
        assert_eq!(parsed.undocumented_bytes_len(), 2);
    }

    #[test]
    fn header_rejects_descriptors_wider_than_point_format() {
        let descriptor =
            ExtraBytesDescriptor::new("value", ExtraBytesDataType::U16).expect("descriptor");
        let extra_bytes = ExtraBytesVlr::from_descriptors([descriptor]).expect("schema");

        let mut builder = Builder::default();
        builder.point_format.extra_bytes = 1;
        builder.vlrs.push(Vlr::try_from(&extra_bytes).expect("VLR"));
        let header = builder.into_header().expect("header");

        assert!(matches!(
            header.extra_bytes_vlr(),
            Err(Error::ExtraBytesMismatch(2, 1))
        ));
    }

    #[test]
    fn typed_vlr_serialization_preserves_scalar_metadata() {
        let descriptor = ExtraBytesDescriptor::new("temperature", ExtraBytesDataType::I16)
            .expect("descriptor")
            .with_description("Degrees Celsius")
            .expect("description")
            .with_no_data(i16::MIN)
            .expect("no data")
            .with_min(-100_i16)
            .expect("minimum")
            .with_max(500_i16)
            .expect("maximum")
            .with_scale(0.1)
            .expect("scale")
            .with_offset(-10.0)
            .expect("offset");
        let extra_bytes = ExtraBytesVlr::from_descriptors([descriptor]).expect("schema");
        let vlr = Vlr::try_from(&extra_bytes).expect("serialize");
        let parsed = ExtraBytesVlr::try_from(&vlr).expect("parse");
        let descriptor = &parsed.descriptors()[0];

        assert_eq!(descriptor.name(), "temperature");
        assert_eq!(descriptor.description(), "Degrees Celsius");
        assert_eq!(descriptor.data_type(), ExtraBytesDataType::I16);
        assert_eq!(descriptor.no_data(), Some(ExtraBytesValue::Signed(-32_768)));
        assert_eq!(descriptor.min(), Some(ExtraBytesValue::Signed(-100)));
        assert_eq!(descriptor.max(), Some(ExtraBytesValue::Signed(500)));
        assert_eq!(descriptor.scale(), 0.1);
        assert_eq!(descriptor.offset(), -10.0);
    }

    #[test]
    fn typed_vlr_serialization_is_canonical() {
        let mut data = vec![0; ExtraBytesVlr::DESCRIPTOR_SIZE];
        data[0] = 42;
        data[2] = ExtraBytesDataType::U8.code();
        data[4..9].copy_from_slice(b"value");
        data[36..40].copy_from_slice(&[1, 2, 3, 4]);
        let parsed = ExtraBytesVlr::try_from(&Vlr {
            user_id: ExtraBytesVlr::USER_ID.to_owned(),
            record_id: ExtraBytesVlr::RECORD_ID,
            data,
            ..Vlr::default()
        })
        .expect("parse descriptor");

        let serialized = Vlr::try_from(&parsed).expect("serialize descriptor");
        assert_eq!(&serialized.data[0..2], &[0, 0]);
        assert_eq!(&serialized.data[36..40], &[0, 0, 0, 0]);
    }

    #[test]
    fn typed_vlr_serialization_rejects_deprecated_arrays() {
        let mut data = vec![0; ExtraBytesVlr::DESCRIPTOR_SIZE];
        data[2] = 11;
        data[4..9].copy_from_slice(b"array");
        let parsed = ExtraBytesVlr::try_from(&Vlr {
            user_id: ExtraBytesVlr::USER_ID.to_owned(),
            record_id: ExtraBytesVlr::RECORD_ID,
            data,
            ..Vlr::default()
        })
        .expect("deprecated arrays remain readable as raw bytes");

        assert!(matches!(
            Vlr::try_from(&parsed),
            Err(Error::UnsupportedExtraBytesDataType(11))
        ));
    }

    #[test]
    fn builder_installs_schema_and_point_setters_encode_values() {
        let scaled = ExtraBytesDescriptor::new("scaled", ExtraBytesDataType::I16)
            .expect("descriptor")
            .with_scale(0.5)
            .expect("scale")
            .with_offset(10.0)
            .expect("offset");
        let raw = ExtraBytesDescriptor::undocumented("raw", 3).expect("raw descriptor");
        let extra_bytes =
            ExtraBytesVlr::from_descriptors([scaled, raw]).expect("Extra Bytes schema");

        let mut builder = Builder::default();
        builder.vlrs.push(Vlr {
            user_id: ExtraBytesVlr::USER_ID.to_owned(),
            record_id: ExtraBytesVlr::RECORD_ID,
            ..Vlr::default()
        });
        builder
            .set_extra_bytes_vlr(&extra_bytes)
            .expect("install schema");
        assert_eq!(builder.point_format.extra_bytes, 5);
        assert_eq!(
            builder
                .vlrs
                .iter()
                .filter(|vlr| vlr.is_extra_bytes())
                .count(),
            1
        );

        let mut point = Point::default();
        extra_bytes.initialize_point(&mut point);
        point
            .set_extra_field(&extra_bytes.descriptors()[0], 20.0_f64)
            .expect("scaled value");
        point
            .set_extra_field_raw(&extra_bytes.descriptors()[1], &[1, 2, 3])
            .expect("raw value");

        assert_eq!(point.extra_bytes, [20, 0, 1, 2, 3]);
        assert_eq!(
            point
                .extra_field(&extra_bytes.descriptors()[0])
                .expect("decoded value"),
            ExtraBytesValue::Float(20.0)
        );
    }

    #[test]
    fn nullable_point_setter_writes_raw_no_data_value() {
        let descriptor = ExtraBytesDescriptor::new("quality", ExtraBytesDataType::U8)
            .expect("descriptor")
            .with_no_data(u8::MAX)
            .expect("no data");
        let extra_bytes =
            ExtraBytesVlr::from_descriptors([descriptor]).expect("Extra Bytes schema");
        let descriptor = &extra_bytes.descriptors()[0];
        let mut point = Point::default();
        extra_bytes.initialize_point(&mut point);

        point
            .set_extra_field_nullable::<u8>(descriptor, None)
            .expect("nullable value");
        assert_eq!(point.extra_bytes, [u8::MAX]);
        assert_eq!(
            point.extra_field(descriptor).expect("decoded value"),
            ExtraBytesValue::Unsigned(u64::from(u8::MAX))
        );
        assert!(point.set_extra_field(descriptor, 256_u16).is_err());
    }

    #[test]
    fn point_data_column_setter_is_typed_and_checks_length() {
        let descriptor =
            ExtraBytesDescriptor::new("value", ExtraBytesDataType::U16).expect("descriptor");
        let extra_bytes = ExtraBytesVlr::from_descriptors([descriptor]).expect("schema");
        let descriptor = &extra_bytes.descriptors()[0];
        let mut format = Format::new(0).expect("point format");
        format.extra_bytes = 2;
        let mut points = PointDataBuilder::new().with_format(format).build();
        let _ = points.resize_for(3);

        points
            .set_extra_column(descriptor, [12_u16, 34, 56])
            .expect("column");
        match points
            .extra_column(descriptor)
            .expect("column")
            .expect("field")
        {
            ExtraBytesColumn::Unsigned(values) => {
                assert_eq!(values.collect::<Vec<_>>(), [12, 34, 56]);
            }
            ExtraBytesColumn::Signed(_) | ExtraBytesColumn::Float(_) => {
                panic!("expected unsigned column")
            }
        }
        assert!(matches!(
            points.set_extra_column(descriptor, [1_u16, 2]),
            Err(Error::ExtraBytesColumnLengthMismatch {
                expected: 3,
                actual: 2
            })
        ));
    }

    #[test]
    fn typed_extra_bytes_roundtrip_through_writer() {
        let descriptor = ExtraBytesDescriptor::new("temperature", ExtraBytesDataType::I16)
            .expect("descriptor")
            .with_scale(0.25)
            .expect("scale")
            .with_offset(100.0)
            .expect("offset");
        let extra_bytes = ExtraBytesVlr::from_descriptors([descriptor]).expect("schema");
        let descriptor = &extra_bytes.descriptors()[0];
        let mut builder = Builder::default();
        builder
            .set_extra_bytes_vlr(&extra_bytes)
            .expect("install schema");
        let header = builder.into_header().expect("header");

        let mut point = Point::default();
        extra_bytes.initialize_point(&mut point);
        point
            .set_extra_field(descriptor, 101.25_f64)
            .expect("temperature");
        let mut writer = Writer::new(Cursor::new(Vec::new()), header).expect("writer");
        writer.write_point(point).expect("write point");

        let mut reader = Reader::new(writer.into_inner().expect("written LAS")).expect("reader");
        let read_extra_bytes = reader
            .header()
            .extra_bytes_vlr()
            .expect("valid Extra Bytes VLR")
            .expect("Extra Bytes VLR");
        let points = reader.read_all().expect("points");
        let point = points.points().next().expect("one point").expect("point");
        assert_eq!(
            point
                .extra_field(&read_extra_bytes.descriptors()[0])
                .expect("decoded value"),
            ExtraBytesValue::Float(101.25)
        );
    }
}
