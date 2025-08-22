use std::io::Write;
use std::marker::PhantomData;

use borsh::BorshSerialize;

#[derive(Clone, Debug, Default)]
pub struct SmallVec<L, T>(Vec<T>, PhantomData<L>);

impl<L, T> SmallVec<L, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<L, T> From<SmallVec<L, T>> for Vec<T> {
    fn from(val: SmallVec<L, T>) -> Self {
        val.0
    }
}

impl<L, T> From<Vec<T>> for SmallVec<L, T> {
    fn from(val: Vec<T>) -> Self {
        Self(val, PhantomData)
    }
}

impl<T: BorshSerialize> BorshSerialize for SmallVec<u8, T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len = u8::try_from(self.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        // Write the length of the vector as u8.
        writer.write_all(&len.to_le_bytes())?;

        // Write the vector elements.
        serialize_slice(&self.0, writer)
    }
}

impl<T: BorshSerialize> BorshSerialize for SmallVec<u16, T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len = u16::try_from(self.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        // Write the length of the vector as u16.
        writer.write_all(&len.to_le_bytes())?;

        // Write the vector elements.
        serialize_slice(&self.0, writer)
    }
}

/// Helper method that is used to serialize a slice of data (without the length marker).
/// Copied from borsh::ser::serialize_slice.
#[inline]
fn serialize_slice<T: BorshSerialize, W: Write>(data: &[T], writer: &mut W) -> std::io::Result<()> {
    if let Some(u8_slice) = T::u8_slice(data) {
        writer.write_all(u8_slice)?;
    } else {
        for item in data {
            item.serialize(writer)?;
        }
    }
    Ok(())
}
