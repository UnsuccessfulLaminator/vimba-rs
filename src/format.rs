#![allow(non_camel_case_types, dead_code)]

use crate::vimba_sys::{VmbPixelFormatType::*, VmbPixelType::*};
use num_derive::FromPrimitive;



const FORMAT_COLOR_MASK: u32     = 0xFF000000;
const FORMAT_BIT_DEPTH_MASK: u32 = 0x00FF0000;
const FORMAT_ID_MASK: u32        = 0x0000FFFF;

#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Eq, FromPrimitive)]
pub enum PixelFormat {
    // Mono formats
    Mono8 = VmbPixelFormatMono8,
    Mono10 = VmbPixelFormatMono10,
    Mono10p = VmbPixelFormatMono10p,
    Mono12 = VmbPixelFormatMono12,
    Mono12Packed = VmbPixelFormatMono12Packed,
    Mono12p = VmbPixelFormatMono12p,
    Mono14 = VmbPixelFormatMono14,
    Mono16 = VmbPixelFormatMono16,

    // Bayer formats
    BayerGR8 = VmbPixelFormatBayerGR8,
    BayerRG8 = VmbPixelFormatBayerRG8,
    BayerGB8 = VmbPixelFormatBayerGB8,
    BayerBG8 = VmbPixelFormatBayerBG8,
    BayerGR10 = VmbPixelFormatBayerGR10,
    BayerRG10 = VmbPixelFormatBayerRG10,
    BayerGB10 = VmbPixelFormatBayerGB10,
    BayerBG10 = VmbPixelFormatBayerBG10,
    BayerGR12 = VmbPixelFormatBayerGR12,
    BayerRG12 = VmbPixelFormatBayerRG12,
    BayerGB12 = VmbPixelFormatBayerGB12,
    BayerBG12 = VmbPixelFormatBayerBG12,
    BayerGR12Packed = VmbPixelFormatBayerGR12Packed,
    BayerRG12Packed = VmbPixelFormatBayerRG12Packed,
    BayerGB12Packed = VmbPixelFormatBayerGB12Packed,
    BayerBG12Packed = VmbPixelFormatBayerBG12Packed,
    BayerGR10p = VmbPixelFormatBayerGR10p,
    BayerRG10p = VmbPixelFormatBayerRG10p,
    BayerGB10p = VmbPixelFormatBayerGB10p,
    BayerBG10p = VmbPixelFormatBayerBG10p,
    BayerGR12p = VmbPixelFormatBayerGR12p,
    BayerRG12p = VmbPixelFormatBayerRG12p,
    BayerGB12p = VmbPixelFormatBayerGB12p,
    BayerBG12p = VmbPixelFormatBayerBG12p,
    BayerGR16 = VmbPixelFormatBayerGR16,
    BayerRG16 = VmbPixelFormatBayerRG16,
    BayerGB16 = VmbPixelFormatBayerGB16,
    BayerBG16 = VmbPixelFormatBayerBG16,

    // RGB formats
    Rgb8 = VmbPixelFormatRgb8,
    Bgr8 = VmbPixelFormatBgr8,
    Rgb10 = VmbPixelFormatRgb10,
    Bgr10 = VmbPixelFormatBgr10,
    Rgb12 = VmbPixelFormatRgb12,
    Bgr12 = VmbPixelFormatBgr12,
    Rgb14 = VmbPixelFormatRgb14,
    Bgr14 = VmbPixelFormatBgr14,
    Rgb16 = VmbPixelFormatRgb16,
    Bgr16 = VmbPixelFormatBgr16,

    // RGBA formats

    // In Vimba's VmbCommonTypes.h header, VmbPixelFormatArgb8 and VmbPixelFormatRgba8
    // are defined to be exactly the same. I cannot for the life of me understand why,
    // as the 2 different names clearly mean a different channel ordering. Anyway, I've
    // commented this line out because the header actually says "(PFNC:RGBa8)" in the
    // comment for this pixel format, so we can just use VmbPixelFormatRgba8.
    //Argb8 = VmbPixelFormatArgb8,

    Rgba8 = VmbPixelFormatRgba8,
    Bgra8 = VmbPixelFormatBgra8,
    Rgba10 = VmbPixelFormatRgba10,
    Bgra10 = VmbPixelFormatBgra10,
    Rgba12 = VmbPixelFormatRgba12,
    Bgra12 = VmbPixelFormatBgra12,
    Rgba14 = VmbPixelFormatRgba14,
    Bgra14 = VmbPixelFormatBgra14,
    Rgba16 = VmbPixelFormatRgba16,
    Bgra16 = VmbPixelFormatBgra16,
    
    // YUV/YCbCr formats
    Yuv411 = VmbPixelFormatYuv411,
    Yuv422 = VmbPixelFormatYuv422,
    Yuv444 = VmbPixelFormatYuv444,
    YCbCr411_8_CbYYCrYY = VmbPixelFormatYCbCr411_8_CbYYCrYY,
    YCbCr422_8_CbYCrY = VmbPixelFormatYCbCr422_8_CbYCrY,
    YCbCr8_CbYCr = VmbPixelFormatYCbCr8_CbYCr
}

impl Default for PixelFormat {
    fn default() -> Self {
        Self::Mono8
    }
}

impl PixelFormat {
    pub fn bits_per_pixel(&self) -> usize {
        const SHIFT: u32 = FORMAT_BIT_DEPTH_MASK.trailing_zeros();

        ((*self as u32 & FORMAT_BIT_DEPTH_MASK) >> SHIFT) as usize
    }

    pub fn is_color(&self) -> bool {
        *self as u32 & VmbPixelColor > 0
    }

    pub fn num_channels(&self) -> usize {
        use PixelFormat::*;

        match self {
            // RGB and YUV formats are all 3-channel
            Rgb8 | Bgr8 | Rgb10 | Bgr10 | Rgb12 | Bgr12 | Rgb14 | Bgr14 | Rgb16 | Bgr16
            | Yuv411 | Yuv422 | Yuv444
            | YCbCr411_8_CbYYCrYY | YCbCr422_8_CbYCrY | YCbCr8_CbYCr => 3,
            
            // RGBA formats are all 4-channel
            Rgba8 | Bgra8 | Rgba10 | Bgra10 | Rgba12 |
            Bgra12 | Rgba14 | Bgra14 | Rgba16 | Bgra16 => 4,

            // Everything else is Mono or Bayer, which are only 1-channel
            _ => 1
        }
    }

    pub fn bits_per_channel(&self) -> usize {
        use PixelFormat::*;

        match self {
            Mono8 => 8,
            Mono10 | Mono10p => 10,
            Mono12 | Mono12Packed | Mono12p => 12,
            Mono14 => 14,
            Mono16 => 16,

            Rgb8 | Bgr8 | Rgba8 | Bgra8 => 8,
            Rgb10 | Bgr10 | Rgba10 | Bgra10 => 10,
            Rgb12 | Bgr12 | Rgba12 | Bgra12 => 12,
            Rgb14 | Bgr14 | Rgba14 | Bgra14 => 14,
            Rgb16 | Bgr16 | Rgba16 | Bgra16 => 16,

            Yuv411 | Yuv422 | Yuv444
            | YCbCr411_8_CbYYCrYY | YCbCr422_8_CbYCrY | YCbCr8_CbYCr => 8,
            
            BayerGR8 | BayerRG8 | BayerGB8 | BayerBG8 => 8,
            BayerGR10 | BayerRG10 | BayerGB10 | BayerBG10 => 10,
            BayerGR10p | BayerRG10p | BayerGB10p | BayerBG10p => 10,
            BayerGR12 | BayerRG12 | BayerGB12 | BayerBG12 => 12,
            BayerGR12p | BayerRG12p | BayerGB12p | BayerBG12p => 12,
            BayerGR12Packed | BayerRG12Packed | BayerGB12Packed | BayerBG12Packed => 12,
            BayerGR16 | BayerRG16 | BayerGB16 | BayerBG16 => 16
        }
    }

    pub fn unpack_to_u16(&self, raw: &[u8]) -> Option<Vec<u16>> {
        use PixelFormat::*;

        let mut out = vec![];

        match self {
            // These are all 2 bytes per component. It appears that little endian
            // is used for all of these, so for maximum portability I'm using
            // u16::from_le_bytes rather than simply recasting [u8] to [u16]
            Mono10 | Mono12 | Mono14 | Mono16
            | Rgb10 | Bgr10 | Rgba10 | Bgra10
            | Rgb12 | Bgr12 | Rgba12 | Bgra12
            | Rgb14 | Bgr14 | Rgba14 | Bgra14
            | Rgb16 | Bgr16 | Rgba16 | Bgra16
            | BayerGR10 | BayerRG10 | BayerGB10 | BayerBG10
            | BayerGR12 | BayerRG12 | BayerGB12 | BayerBG12
            | BayerGR16 | BayerRG16 | BayerGB16 | BayerBG16
            => {
                out.resize(raw.len()/2, 0);

                for (i, x) in out.iter_mut().enumerate() {
                    *x = u16::from_le_bytes([raw[i*2], raw[i*2+1]]);
                }
            },

            Mono10p
            | BayerGR10p | BayerRG10p | BayerGB10p | BayerBG10p
            | BayerGR12p | BayerRG12p | BayerGB12p | BayerBG12p 
            | BayerGR12Packed | BayerRG12Packed | BayerGB12Packed | BayerBG12Packed
            => {
                let channel_bits = self.bits_per_channel();
                out.resize((raw.len()*8)/channel_bits, 0);

                for (i, x) in out.iter_mut().enumerate() {
                    let start_bit = i*channel_bits;
                    let start_byte = start_bit/8;
                    let rem_bits = start_bit % 8;
                    let first = raw[start_byte];
                    let next = raw[start_byte+1];

                    *x = u16::from_le_bytes([first, next]) >> rem_bits;
                }
            }
            
            // Everything else has a number of bits < 8 or >= 16
            _ => return None
        }

        Some(out)
    }
}
