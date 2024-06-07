//! Pingxelflut packet format structures, most important of which the [`Packet`] enum.
//!
//! Refer to the [README](../../README.md) for the protocol specification.

use bytemuck::checked::cast;
use bytemuck::checked::try_cast;
use rgb::ComponentSlice;
use rgb::RGB8;
use rgb::RGBA8;

/// A Pingxelflut packet.
#[derive(Debug, Clone, Copy)]
pub enum Packet {
    /// A size request, type `aa`.
    SizeRequest,
    /// A size response, type `bb`.
    SizeResponse { width: u16, height: u16 },
    /// A pixel set request, type `cc`
    SetPixel { x: u16, y: u16, color: Color },
}

pub type Color = RGBA8;
pub const COLOR_SIZE: usize = 4;

pub fn color_from_rgb(vec: [u8; 3]) -> Color {
    cast::<_, RGB8>(vec).alpha(0xff)
}
pub fn color_from_rgba(vec: [u8; 4]) -> Color {
    cast::<_, RGBA8>(vec)
}

impl Packet {
    pub const SIZE_REQUEST_ID: u8 = 0xaa;
    pub const SIZE_RESPONSE_ID: u8 = 0xbb;
    pub const SET_PIXEL_ID: u8 = 0xcc;

    /// Parse a packet from the start of the provided binary representation.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let kind = bytes.first()?;
        match kind {
            0xaa => Some(Self::SizeRequest),
            0xbb => {
                let width = u16::from_be_bytes(bytes.get(1..=2)?.try_into().unwrap());
                let height = u16::from_be_bytes(bytes.get(3..=4)?.try_into().unwrap());
                Some(Self::SizeResponse { width, height })
            }
            0xcc => {
                let x = u16::from_be_bytes(bytes.get(1..=2)?.try_into().unwrap());
                let y = u16::from_be_bytes(bytes.get(3..=4)?.try_into().unwrap());
                let color_slice = bytes.get(5..)?;
                // get an rgb or rgba color, depending on how large the remaining slice is
                let color = <[u8; 4]>::try_from(color_slice)
                    .ok()
                    .and_then(|color| try_cast::<_, RGBA8>(color).ok())
                    .or_else(|| {
                        <[u8; 3]>::try_from(color_slice)
                            .ok()
                            .and_then(|color| try_cast::<_, RGB8>(color).ok())
                            .map(|color| color.alpha(0xff))
                    })?;
                Some(Self::SetPixel { x, y, color })
            }
            _ => None,
        }
    }

    /// Write the packet data to the start of a provided buffer.
    /// Returns the number of written bytes, or None if the buffer wasn’t large enough.
    pub fn write_to(&self, buffer: &mut [u8]) -> Option<usize> {
        Some(match self {
            Packet::SizeRequest => {
                buffer.get_mut(0).map(|x| *x = Self::SIZE_REQUEST_ID)?;
                1
            }
            Packet::SizeResponse { width, height } => {
                buffer.get_mut(0).map(|x| *x = Self::SIZE_RESPONSE_ID)?;
                buffer
                    .get_mut(1..=2)
                    .map(|x| x.copy_from_slice(&width.to_be_bytes()))?;
                buffer
                    .get_mut(3..=4)
                    .map(|x| x.copy_from_slice(&height.to_be_bytes()))?;
                5
            }
            Packet::SetPixel { x, y, color } => {
                buffer.get_mut(0).map(|x| *x = Self::SET_PIXEL_ID)?;
                buffer
                    .get_mut(1..=2)
                    .map(|val| val.copy_from_slice(&x.to_be_bytes()))?;
                buffer
                    .get_mut(3..=4)
                    .map(|x| x.copy_from_slice(&y.to_be_bytes()))?;
                5 + if color.a != 0xff {
                    buffer.get_mut(5..9)?.copy_from_slice(color.as_slice());
                    4
                } else {
                    buffer
                        .get_mut(5..8)?
                        .copy_from_slice(color.rgb().as_slice());
                    3
                }
            }
        })
    }

    /// Convert the packet to its byte representation.
    #[cfg(feature = "std")]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = vec![0; 9];
        let length = self.write_to(&mut buffer).unwrap();
        buffer.truncate(length);
        buffer
    }
}
