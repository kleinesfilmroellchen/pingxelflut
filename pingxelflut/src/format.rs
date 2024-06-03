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
    /// Parse a packet from the start of the provided binary representation.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let kind = bytes[0];
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
    /// Returns the number of written bytes.
    ///
    /// # Panics
    ///
    /// Panics if the provided buffer is not large enough.
    pub fn write_to(&self, buffer: &mut [u8]) -> usize {
        match self {
            Packet::SizeRequest => {
                buffer[0] = 0xaa;
                1
            }
            Packet::SizeResponse { width, height } => {
                buffer[0] = 0xbb;
                buffer[1..=2].copy_from_slice(&width.to_be_bytes());
                buffer[3..=4].copy_from_slice(&height.to_be_bytes());
                5
            }
            Packet::SetPixel { x, y, color } => {
                buffer[0] = 0xcc;
                buffer[1..=2].copy_from_slice(&x.to_be_bytes());
                buffer[3..=4].copy_from_slice(&y.to_be_bytes());
                let color_size = color.write_to(&mut buffer[5..]);
                5 + color_size
            }
        }
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
    pub fn from_bytes(bytes: &[u8]) -> Option<Color> {
        match bytes {
            [red, green, blue] => Some(Color {
                red: *red,
                green: *green,
                blue: *blue,
                alpha: None,
            }),
            [red, green, blue, alpha] => Some(Color {
                red: *red,
                green: *green,
                blue: *blue,
                alpha: Some(*alpha),
            }),
            _ => None,
        }
    }

    /// Returns an alpha value, possibly set to fully opaque if the color doesn’t have a custom alpha value.
    pub fn alpha(&self) -> u8 {
        self.alpha.unwrap_or(0xff)
    }

    /// Write the color data to the start of a provided buffer.
    /// Returns the number of written bytes.
    ///
    /// # Panics
    ///
    /// Panics if the provided buffer is not large enough.
    pub fn write_to(&self, buffer: &mut [u8]) -> usize {
        buffer[0] = self.red;
        buffer[1] = self.green;
        buffer[2] = self.blue;
        if let Some(alpha) = self.alpha {
            buffer[3] = alpha;
            4
        } else {
            3
        }
    }

    /// Convert the color to its byte representation.
    pub fn to_bytes(self) -> Vec<u8> {
        let mut buffer = vec![self.red, self.green, self.blue];
        if let Some(alpha) = self.alpha {
            buffer.push(alpha);
        }
        buffer
    }
}
