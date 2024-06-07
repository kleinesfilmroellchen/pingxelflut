//! Pingxelflut packet format structures, most important of which the [`Packet`] enum.
//!
//! Refer to the [README](../../README.md) for the protocol specification.

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
                let color = Color::from_bytes(bytes.get(5..)?)?;
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
                buffer.get_mut(1..=2).map(|x| x.copy_from_slice(&width.to_be_bytes()))?;
                buffer.get_mut(3..=4).map(|x| x.copy_from_slice(&height.to_be_bytes()))?;
                5
            }
            Packet::SetPixel { x, y, color } => {
                buffer.get_mut(0).map(|x| *x = Self::SET_PIXEL_ID)?;
                buffer.get_mut(1..=2).map(|val| val.copy_from_slice(&x.to_be_bytes()))?;
                buffer.get_mut(3..=4).map(|x| x.copy_from_slice(&y.to_be_bytes()))?;
                let color_size = color.write_to(buffer.get_mut(5..)?);
                5 + color_size
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

/// A Pixelflut color.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
    /// Transparency, optional.
    pub alpha: Option<u8>,
}

impl Color {
    /// Create a color struct from three (RGB) or four (RGBA) bytes.
    ///
    /// [`None`] is returned if the input bytes are not exactly 3 or 4 in length.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [red, green, blue] => Some(Self {
                red: *red,
                green: *green,
                blue: *blue,
                alpha: None,
            }),
            [red, green, blue, alpha] => Some(Self {
                red: *red,
                green: *green,
                blue: *blue,
                alpha: Some(*alpha),
            }),
            _ => None,
        }
    }

    /// Create a color struct from three RGB bytes.
    #[inline]
    pub fn from_rgb([red, green, blue]: [u8; 3]) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: None,
        }
    }

    /// Create a color struct from four RGBA bytes.
    #[inline]
    pub fn from_rgba([red, green, blue, alpha]: [u8; 4]) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: Some(alpha),
        }
    }

    /// Returns an alpha value, possibly set to fully opaque if the color doesn’t have a custom alpha value.
    pub fn alpha(&self) -> u8 {
        self.alpha.unwrap_or(0xff)
    }

    /// Write the color data to the start of a provided buffer.
    /// Returns the number of written bytes.
    ///
    /// This function will stop writing anything if the buffer is not large enough.
    /// It will still return the number of bytes that would have been written in theory.
    pub fn write_to(&self, buffer: &mut [u8]) -> usize {
        buffer.get_mut(0).map(|x| *x = self.red);
        buffer.get_mut(1).map(|x| *x = self.green);
        buffer.get_mut(2).map(|x| *x = self.blue);
        if let Some(alpha) = self.alpha {
            buffer.get_mut(3).map(|x| *x = alpha);
            4
        } else {
            3
        }
    }

    /// Convert the color to its byte representation.
    #[cfg(feature = "std")]
    pub fn to_bytes(self) -> Vec<u8> {
        let mut buffer = vec![self.red, self.green, self.blue];
        if let Some(alpha) = self.alpha {
            buffer.push(alpha);
        }
        buffer
    }
}
