#![allow(dead_code)]
// This is a hot mess copied from https://github.com/PistonDevelopers/freetype-rs
// It's a nice abstraction over freetype, but PistonDevelopers/freetype-rs links 
// against different freetype-sys then harfbuzz_rs so they can't be used toghether
// This code copies stuff from piston freetype lib but uses the freetype-sys that
// harfbuzz_rs links against. It's very very far from an ideal solution.

use std::fmt;
use std::ptr;
use std::slice;
use std::error;
use std::rc::Rc;
use std::ffi::CStr;

use bitflags::bitflags;
use freetype::succeeded;
use freetype::freetype as ffi;

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum KerningMode {
    KerningDefault  = ffi::FT_Kerning_Mode::FT_KERNING_DEFAULT as u32,
    KerningUnfitted = ffi::FT_Kerning_Mode::FT_KERNING_UNFITTED as u32,
    KerningUnscaled = ffi::FT_Kerning_Mode::FT_KERNING_UNSCALED as u32
}

bitflags! {
    pub struct LoadFlag: u32 {
        const DEFAULT                    = ffi::FT_LOAD_DEFAULT;
        const NO_SCALE                   = ffi::FT_LOAD_NO_SCALE;
        const NO_HINTING                 = ffi::FT_LOAD_NO_HINTING;
        const RENDER                     = ffi::FT_LOAD_RENDER;
        const NO_BITMAP                  = ffi::FT_LOAD_NO_BITMAP;
        const VERTICAL_LAYOUT            = ffi::FT_LOAD_VERTICAL_LAYOUT;
        const FORCE_AUTOHINT             = ffi::FT_LOAD_FORCE_AUTOHINT;
        const CROP_BITMAP                = ffi::FT_LOAD_CROP_BITMAP;
        const PEDANTIC                   = ffi::FT_LOAD_PEDANTIC;
        const IGNORE_GLOBAL_ADVANCE_WITH = ffi::FT_LOAD_IGNORE_GLOBAL_ADVANCE_WIDTH;
        const NO_RECURSE                 = ffi::FT_LOAD_NO_RECURSE;
        const IGNORE_TRANSFORM           = ffi::FT_LOAD_IGNORE_TRANSFORM;
        const MONOCHROME                 = ffi::FT_LOAD_MONOCHROME;
        const LINEAR_DESIGN              = ffi::FT_LOAD_LINEAR_DESIGN;
        const NO_AUTOHINT                = ffi::FT_LOAD_NO_AUTOHINT;
        /*
        const TARGET_NORMAL              = ffi::FT_LOAD_TARGET_NORMAL;
        const TARGET_LIGHT               = ffi::FT_LOAD_TARGET_LIGHT;
        const TARGET_MONO                = ffi::FT_LOAD_TARGET_MONO;
        const TARGET_LCD                 = ffi::FT_LOAD_TARGET_LCD;
        const TARGET_LCD_V               = ffi::FT_LOAD_TARGET_LCD_V;*/
        const COLOR                      = ffi::FT_LOAD_COLOR;
    }
}

pub type FtResult<T> = Result<T, Error>;

pub struct Library {
    raw: ffi::FT_Library
}

impl Library {
	
	pub fn init() -> FtResult<Self> {
        let mut raw = ptr::null_mut();

        let err = unsafe {
            ffi::FT_Init_FreeType(&mut raw)
        };
        
        if succeeded(err) {
            unsafe {
                ffi::FT_Add_Default_Modules(raw);
            }
            
            Ok(Library {
                raw: raw
            })
        } else {
            Err(err.into())
        }
    }
    
    pub fn new_memory_face<T>(&self, buffer: T, face_index: isize) -> FtResult<Face>
    where
        T: Into<Rc<Vec<u8>>>
    {
        let mut face = ptr::null_mut();
        let buffer = buffer.into();

        let err = unsafe {
            ffi::FT_New_Memory_Face(self.raw, buffer.as_ptr(), buffer.len() as ffi::FT_Long, face_index as ffi::FT_Long, &mut face)
        };
        
        if succeeded(err) {
            Ok(unsafe { Face::from_raw(self.raw, face, Some(buffer)) })
        } else {
            Err(err.into())
        }
    }
	
}

impl Drop for Library {
    fn drop(&mut self) {
        let err = unsafe {
            ffi::FT_Done_FreeType(self.raw)
        };
        
        if !succeeded(err) {
            panic!("Failed to drop freetype library")
        }
    }
}

#[derive(Eq, PartialEq, Hash)]
pub struct Face {
    library_raw: ffi::FT_Library,
    raw: ffi::FT_Face,
    glyph: GlyphSlot,
    bytes: Option<Rc<Vec<u8>>>
}

impl Face {
	pub unsafe fn from_raw(library_raw: ffi::FT_Library, raw: ffi::FT_Face, bytes: Option<Rc<Vec<u8>>>) -> Self {
        ffi::FT_Reference_Library(library_raw);
        
        Face {
            library_raw: library_raw,
            raw: raw,
            glyph: GlyphSlot::from_raw(library_raw, (*raw).glyph),
            bytes: bytes,
        }
    }
    
    #[inline(always)]
    pub fn glyph(&self) -> &GlyphSlot {
        &self.glyph
    }
    
    pub fn get_char_index(&self, charcode: usize) -> u32 {
        unsafe {
            ffi::FT_Get_Char_Index(self.raw, charcode as ffi::FT_ULong)
        }
    }
    
    pub fn load_glyph(&self, glyph_index: u32, load_flags: LoadFlag) -> FtResult<()> {
        let err = unsafe {
            ffi::FT_Load_Glyph(self.raw, glyph_index, load_flags.bits as i32)
        };
        
        if succeeded(err) {
            Ok(())
        } else {
            Err(err.into())
        }
    }
    
    pub fn get_kerning(&self, left_char_index: u32, right_char_index: u32, kern_mode: KerningMode) -> FtResult<ffi::FT_Vector> {
        let mut vec = ffi::FT_Vector { x: 0, y: 0 };

        let err = unsafe {
            ffi::FT_Get_Kerning(self.raw, left_char_index, right_char_index,
                                kern_mode as u32, &mut vec)
        };
        
        if succeeded(err) {
            Ok(vec)
        } else {
            Err(err.into())
        }
    }

    
    pub fn postscript_name(&self) -> Option<String> {
        let face_name = unsafe { ffi::FT_Get_Postscript_Name(self.raw) };
        
        if face_name.is_null() {
            None
        } else {
            let face_name = unsafe {
                CStr::from_ptr(face_name as *const _).to_bytes().to_vec()
            };
            String::from_utf8(face_name).ok()
        }
    }
    
    pub fn set_char_size(&self, char_width: isize, char_height: isize, horz_resolution: u32, vert_resolution: u32) -> FtResult<()> {
        let err = unsafe {
            ffi::FT_Set_Char_Size(self.raw, char_width as ffi::FT_F26Dot6,
                                  char_height as ffi::FT_F26Dot6, horz_resolution,
                                  vert_resolution)
        };
        
        if succeeded(err) {
            Ok(())
        } else {
            Err(err.into())
        }
    }
    
    pub fn set_pixel_sizes(&self, pixel_width: u32, pixel_height: u32) -> FtResult<()> {
        let err = unsafe {
            ffi::FT_Set_Pixel_Sizes(self.raw, pixel_width, pixel_height)
        };
        
        if succeeded(err) {
            Ok(())
        } else {
            Err(err.into())
        }
    }
    
    pub fn size_metrics(&self) -> Option<ffi::FT_Size_Metrics> {
        if self.raw.is_null() {
            None
        } else {
            let size = unsafe { (*self.raw).size };
            
            if size.is_null() {
                None
            } else {
                Some(unsafe { (*size).metrics })
            }
        }
    }
    
    #[inline(always)]
    pub fn raw(&self) -> &ffi::FT_FaceRec {
        unsafe {
            &*self.raw
        }
    }
    
    #[inline(always)]
    pub fn raw_mut(&mut self) -> &mut ffi::FT_FaceRec {
        unsafe {
            &mut *self.raw
        }
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        let err = unsafe {
            ffi::FT_Done_Face(self.raw)
        };
        
        if !succeeded(err) {
            panic!("Failed to drop face");
        }
        
        let err = unsafe {
            ffi::FT_Done_Library(self.library_raw)
        };
        
        if !succeeded(err) {
            panic!("Failed to drop library")
        }
        
        self.bytes = None;
    }
}

/// A struct encapsulating the space for a glyph within a `Library`
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct GlyphSlot {
    library_raw: ffi::FT_Library,
    raw: ffi::FT_GlyphSlot
}

impl GlyphSlot {
    /// Create a `GlyphSlot` from its constituent C parts
    pub unsafe fn from_raw(library_raw: ffi::FT_Library, raw: ffi::FT_GlyphSlot) -> Self {
        GlyphSlot {
            library_raw: library_raw,
            raw: raw
        }
    }
    
    /// The bitmap's left bearing expressed in integer pixels. Only valid if the format is
    /// FT_GLYPH_FORMAT_BITMAP, this is, if the glyph slot contains a bitmap.
    #[inline(always)]
    pub fn bitmap_left(&self) -> i32 {
        unsafe {
            (*self.raw).bitmap_left
        }
    }

    /// The bitmap's top bearing expressed in integer pixels. Remember that this is the distance
    /// from the baseline to the top-most glyph scanline, upwards y coordinates being positive.
    #[inline(always)]
    pub fn bitmap_top(&self) -> i32 {
        unsafe {
            (*self.raw).bitmap_top
        }
    }
    
    /// This shorthand is, depending on FT_LOAD_IGNORE_TRANSFORM, the transformed (hinted) advance
    /// width for the glyph, in 26.6 fractional pixel format. As specified with
    /// FT_LOAD_VERTICAL_LAYOUT, it uses either the ‘horiAdvance’ or the ‘vertAdvance’ value of
    /// ‘metrics’ field.
    #[inline(always)]
    pub fn advance(&self) -> ffi::FT_Vector {
        unsafe {
            (*self.raw).advance
        }
    }
    
    #[inline(always)]
    pub fn bitmap(&self) -> Bitmap {
        unsafe { Bitmap::from_raw(&(*self.raw).bitmap) }
    }
}

#[allow(missing_copy_implementations)]
pub struct Bitmap {
    raw: *const ffi::FT_Bitmap
}

impl Bitmap {
    pub unsafe fn from_raw(raw: *const ffi::FT_Bitmap) -> Self {
        Bitmap {
            raw: raw
        }
    }
    
    /// A typeless pointer to the bitmap buffer. This value should be aligned
    /// on 32-bit boundaries in most cases.
    pub fn buffer(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                (*self.raw).buffer,
                (self.pitch().abs() * self.rows() as i32) as usize
            )
        }
    }

    /// The number of pixels in bitmap row.
    pub fn width(&self) -> u32 {
        unsafe {
            (*self.raw).width
        }
    }

    /// The number of bitmap rows.
    pub fn rows(&self) -> u32 {
        unsafe {
            (*self.raw).rows
        }
    }
    
    pub fn pitch(&self) -> i32 {
        unsafe {
            (*self.raw).pitch
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum Error {
	Ok                          = ffi::FT_Err_Ok as i32,
	CannotOpenResource          = ffi::FT_Err_Cannot_Open_Resource as i32,
    UnknownFileFormat           = ffi::FT_Err_Unknown_File_Format as i32,
    InvalidFileFormat           = ffi::FT_Err_Invalid_File_Format as i32,
    InvalidVersion              = ffi::FT_Err_Invalid_Version as i32,
    LowerModuleVersion          = ffi::FT_Err_Lower_Module_Version as i32,
    InvalidArgument             = ffi::FT_Err_Invalid_Argument as i32,
    UnimplementedFeature        = ffi::FT_Err_Unimplemented_Feature as i32,
    InvalidTable                = ffi::FT_Err_Invalid_Table as i32,
    InvalidOffset               = ffi::FT_Err_Invalid_Offset as i32,
    ArrayTooLarge               = ffi::FT_Err_Array_Too_Large as i32,
    MissingModule               = ffi::FT_Err_Missing_Module as i32,
    MissingProperty             = ffi::FT_Err_Missing_Property as i32,
    InvalidGlyphIndex           = ffi::FT_Err_Invalid_Glyph_Index as i32,
    InvalidCharacterCode        = ffi::FT_Err_Invalid_Character_Code as i32,
    InvalidGlyphFormat          = ffi::FT_Err_Invalid_Glyph_Format as i32,
    CannotRenderGlyph           = ffi::FT_Err_Cannot_Render_Glyph as i32,
    InvalidOutline              = ffi::FT_Err_Invalid_Outline as i32,
    InvalidComposite            = ffi::FT_Err_Invalid_Composite as i32,
    TooManyHints                = ffi::FT_Err_Too_Many_Hints as i32,
    InvalidPixelSize            = ffi::FT_Err_Invalid_Pixel_Size as i32,
    InvalidHandle               = ffi::FT_Err_Invalid_Handle as i32,
    InvalidLibraryHandle        = ffi::FT_Err_Invalid_Library_Handle as i32,
    InvalidDriverHandle         = ffi::FT_Err_Invalid_Driver_Handle as i32,
    InvalidFaceHandle           = ffi::FT_Err_Invalid_Face_Handle as i32,
    InvalidSizeHandle           = ffi::FT_Err_Invalid_Size_Handle as i32,
    InvalidSlotHandle           = ffi::FT_Err_Invalid_Slot_Handle as i32,
    InvalidCharMapHandle        = ffi::FT_Err_Invalid_CharMap_Handle as i32,
    InvalidCacheHandle          = ffi::FT_Err_Invalid_Cache_Handle as i32,
    InvalidStreamHandle         = ffi::FT_Err_Invalid_Stream_Handle as i32,
    TooManyDrivers              = ffi::FT_Err_Too_Many_Drivers as i32,
    TooManyExtensions           = ffi::FT_Err_Too_Many_Extensions as i32,
    OutOfMemory                 = ffi::FT_Err_Out_Of_Memory as i32,
    UnlistedObject              = ffi::FT_Err_Unlisted_Object as i32,
    CannotOpenStream            = ffi::FT_Err_Cannot_Open_Stream as i32,
    InvalidStreamSeek           = ffi::FT_Err_Invalid_Stream_Seek as i32,
    InvalidStreamSkip           = ffi::FT_Err_Invalid_Stream_Skip as i32,
    InvalidStreamRead           = ffi::FT_Err_Invalid_Stream_Read as i32,
    InvalidStreamOperation      = ffi::FT_Err_Invalid_Stream_Operation as i32,
    InvalidFrameOperation       = ffi::FT_Err_Invalid_Frame_Operation as i32,
    NestedFrameAccess           = ffi::FT_Err_Nested_Frame_Access as i32,
    InvalidFrameRead            = ffi::FT_Err_Invalid_Frame_Read as i32,
    RasterUninitialized         = ffi::FT_Err_Raster_Uninitialized as i32,
    RasterCorrupted             = ffi::FT_Err_Raster_Corrupted as i32,
    RasterOverflow              = ffi::FT_Err_Raster_Overflow as i32,
    RasterNegativeHeight        = ffi::FT_Err_Raster_Negative_Height as i32,
    TooManyCaches               = ffi::FT_Err_Too_Many_Caches as i32,
    InvalidOpcode               = ffi::FT_Err_Invalid_Opcode as i32,
    TooFewArguments             = ffi::FT_Err_Too_Few_Arguments as i32,
    StackOverflow               = ffi::FT_Err_Stack_Overflow as i32,
    CodeOverflow                = ffi::FT_Err_Code_Overflow as i32,
    BadArgument                 = ffi::FT_Err_Bad_Argument as i32,
    DivideByZero                = ffi::FT_Err_Divide_By_Zero as i32,
    InvalidReference            = ffi::FT_Err_Invalid_Reference as i32,
    DebugOpCode                 = ffi::FT_Err_Debug_OpCode as i32,
    ENDFInExecStream            = ffi::FT_Err_ENDF_In_Exec_Stream as i32,
    NestedDEFS                  = ffi::FT_Err_Nested_DEFS as i32,
    InvalidCodeRange            = ffi::FT_Err_Invalid_CodeRange as i32,
    ExecutionTooLong            = ffi::FT_Err_Execution_Too_Long as i32,
    TooManyFunctionDefs         = ffi::FT_Err_Too_Many_Function_Defs as i32,
    TooManyInstructionDefs      = ffi::FT_Err_Too_Many_Instruction_Defs as i32,
    TableMissing                = ffi::FT_Err_Table_Missing as i32,
    HorizHeaderMissing          = ffi::FT_Err_Horiz_Header_Missing as i32,
    LocationsMissing            = ffi::FT_Err_Locations_Missing as i32,
    NameTableMissing            = ffi::FT_Err_Name_Table_Missing as i32,
    CMapTableMissing            = ffi::FT_Err_CMap_Table_Missing as i32,
    HmtxTableMissing            = ffi::FT_Err_Hmtx_Table_Missing as i32,
    PostTableMissing            = ffi::FT_Err_Post_Table_Missing as i32,
    InvalidHorizMetrics         = ffi::FT_Err_Invalid_Horiz_Metrics as i32,
    InvalidCharMapFormat        = ffi::FT_Err_Invalid_CharMap_Format as i32,
    InvalidPPem                 = ffi::FT_Err_Invalid_PPem as i32,
    InvalidVertMetrics          = ffi::FT_Err_Invalid_Vert_Metrics as i32,
    CouldNotFindContext         = ffi::FT_Err_Could_Not_Find_Context as i32,
    InvalidPostTableFormat      = ffi::FT_Err_Invalid_Post_Table_Format as i32,
    InvalidPostTable            = ffi::FT_Err_Invalid_Post_Table as i32,
    Syntax                      = ffi::FT_Err_Syntax_Error as i32,
    StackUnderflow              = ffi::FT_Err_Stack_Underflow as i32,
    Ignore                      = ffi::FT_Err_Ignore as i32,
    NoUnicodeGlyphName          = ffi::FT_Err_No_Unicode_Glyph_Name as i32,
    MissingStartfontField       = ffi::FT_Err_Missing_Startfont_Field as i32,
    MissingFontField            = ffi::FT_Err_Missing_Font_Field as i32,
    MissingSizeField            = ffi::FT_Err_Missing_Size_Field as i32,
    MissingFontboundingboxField = ffi::FT_Err_Missing_Fontboundingbox_Field as i32,
    MissingCharsField           = ffi::FT_Err_Missing_Chars_Field as i32,
    MissingStartcharField       = ffi::FT_Err_Missing_Startchar_Field as i32,
    MissingEncodingField        = ffi::FT_Err_Missing_Encoding_Field as i32,
    MissingBbxField             = ffi::FT_Err_Missing_Bbx_Field as i32,
    BbxTooBig                   = ffi::FT_Err_Bbx_Too_Big as i32,
    CorruptedFontHeader         = ffi::FT_Err_Corrupted_Font_Header as i32,
    CorruptedFontGlyphs         = ffi::FT_Err_Corrupted_Font_Glyphs as i32,
    Max                         = ffi::FT_Err_Max as i32,
    UnexpectedPixelMode,
    InvalidPath,
	Unknown
}

impl From<i32> for Error {
    fn from(err: i32) -> Self {
		if ffi::FT_Err_Ok as i32 == err { Error::Ok }
		else if ffi::FT_Err_Cannot_Open_Resource as i32 == err { Error::CannotOpenResource }
		else if ffi::FT_Err_Unknown_File_Format as i32 == err { Error::UnknownFileFormat }
		else if ffi::FT_Err_Invalid_File_Format as i32 == err { Error::InvalidFileFormat }
		else if ffi::FT_Err_Invalid_Version as i32 == err { Error::InvalidVersion }
		else if ffi::FT_Err_Lower_Module_Version as i32 == err { Error::LowerModuleVersion }
		else if ffi::FT_Err_Invalid_Argument as i32 == err { Error::InvalidArgument }
		else if ffi::FT_Err_Unimplemented_Feature as i32 == err { Error::UnimplementedFeature }
		else if ffi::FT_Err_Invalid_Table as i32 == err { Error::InvalidTable }
		else if ffi::FT_Err_Invalid_Offset as i32 == err { Error::InvalidOffset }
		else if ffi::FT_Err_Array_Too_Large as i32 == err { Error::ArrayTooLarge }
		else if ffi::FT_Err_Missing_Module as i32 == err { Error::MissingModule }
		else if ffi::FT_Err_Missing_Property as i32 == err { Error::MissingProperty }
		else if ffi::FT_Err_Invalid_Glyph_Index as i32 == err { Error::InvalidGlyphIndex }
		else if ffi::FT_Err_Invalid_Character_Code as i32 == err { Error::InvalidCharacterCode }
		else if ffi::FT_Err_Invalid_Glyph_Format as i32 == err { Error::InvalidGlyphFormat }
		else if ffi::FT_Err_Cannot_Render_Glyph as i32 == err { Error::CannotRenderGlyph }
		else if ffi::FT_Err_Invalid_Outline as i32 == err { Error::InvalidOutline }
		else if ffi::FT_Err_Invalid_Composite as i32 == err { Error::InvalidComposite }
		else if ffi::FT_Err_Too_Many_Hints as i32 == err { Error::TooManyHints }
		else if ffi::FT_Err_Invalid_Pixel_Size as i32 == err { Error::InvalidPixelSize }
		else if ffi::FT_Err_Invalid_Handle as i32 == err { Error::InvalidHandle }
		else if ffi::FT_Err_Invalid_Library_Handle as i32 == err { Error::InvalidLibraryHandle }
		else if ffi::FT_Err_Invalid_Driver_Handle as i32 == err { Error::InvalidDriverHandle }
		else if ffi::FT_Err_Invalid_Face_Handle as i32 == err { Error::InvalidFaceHandle }
		else if ffi::FT_Err_Invalid_Size_Handle as i32 == err { Error::InvalidSizeHandle }
		else if ffi::FT_Err_Invalid_Slot_Handle as i32 == err { Error::InvalidSlotHandle }
		else if ffi::FT_Err_Invalid_CharMap_Handle as i32 == err { Error::InvalidCharMapHandle }
		else if ffi::FT_Err_Invalid_Cache_Handle as i32 == err { Error::InvalidCacheHandle }
		else if ffi::FT_Err_Invalid_Stream_Handle as i32 == err { Error::InvalidStreamHandle }
		else if ffi::FT_Err_Too_Many_Drivers as i32 == err { Error::TooManyDrivers }
		else if ffi::FT_Err_Too_Many_Extensions as i32 == err { Error::TooManyExtensions }
		else if ffi::FT_Err_Out_Of_Memory as i32 == err { Error::OutOfMemory }
		else if ffi::FT_Err_Unlisted_Object as i32 == err { Error::UnlistedObject }
		else if ffi::FT_Err_Cannot_Open_Stream as i32 == err { Error::CannotOpenStream }
		else if ffi::FT_Err_Invalid_Stream_Seek as i32 == err { Error::InvalidStreamSeek }
		else if ffi::FT_Err_Invalid_Stream_Skip as i32 == err { Error::InvalidStreamSkip }
		else if ffi::FT_Err_Invalid_Stream_Read as i32 == err { Error::InvalidStreamRead }
		else if ffi::FT_Err_Invalid_Stream_Operation as i32 == err { Error::InvalidStreamOperation }
		else if ffi::FT_Err_Invalid_Frame_Operation as i32 == err { Error::InvalidFrameOperation }
		else if ffi::FT_Err_Nested_Frame_Access as i32 == err { Error::NestedFrameAccess }
		else if ffi::FT_Err_Invalid_Frame_Read as i32 == err { Error::InvalidFrameRead }
		else if ffi::FT_Err_Raster_Uninitialized as i32 == err { Error::RasterUninitialized }
		else if ffi::FT_Err_Raster_Corrupted as i32 == err { Error::RasterCorrupted }
		else if ffi::FT_Err_Raster_Overflow as i32 == err { Error::RasterOverflow }
		else if ffi::FT_Err_Raster_Negative_Height as i32 == err { Error::RasterNegativeHeight }
		else if ffi::FT_Err_Too_Many_Caches as i32 == err { Error::TooManyCaches }
		else if ffi::FT_Err_Invalid_Opcode as i32 == err { Error::InvalidOpcode }
		else if ffi::FT_Err_Too_Few_Arguments as i32 == err { Error::TooFewArguments }
		else if ffi::FT_Err_Stack_Overflow as i32 == err { Error::StackOverflow }
		else if ffi::FT_Err_Code_Overflow as i32 == err { Error::CodeOverflow }
		else if ffi::FT_Err_Bad_Argument as i32 == err { Error::BadArgument }
		else if ffi::FT_Err_Divide_By_Zero as i32 == err { Error::DivideByZero }
		else if ffi::FT_Err_Invalid_Reference as i32 == err { Error::InvalidReference }
		else if ffi::FT_Err_Debug_OpCode as i32 == err { Error::DebugOpCode }
		else if ffi::FT_Err_ENDF_In_Exec_Stream as i32 == err { Error::ENDFInExecStream }
		else if ffi::FT_Err_Nested_DEFS as i32 == err { Error::NestedDEFS }
		else if ffi::FT_Err_Invalid_CodeRange as i32 == err { Error::InvalidCodeRange }
		else if ffi::FT_Err_Execution_Too_Long as i32 == err { Error::ExecutionTooLong }
		else if ffi::FT_Err_Too_Many_Function_Defs as i32 == err { Error::TooManyFunctionDefs }
		else if ffi::FT_Err_Too_Many_Instruction_Defs as i32 == err { Error::TooManyInstructionDefs }
		else if ffi::FT_Err_Table_Missing as i32 == err { Error::TableMissing }
		else if ffi::FT_Err_Horiz_Header_Missing as i32 == err { Error::HorizHeaderMissing }
		else if ffi::FT_Err_Locations_Missing as i32 == err { Error::LocationsMissing }
		else if ffi::FT_Err_Name_Table_Missing as i32 == err { Error::NameTableMissing }
		else if ffi::FT_Err_CMap_Table_Missing as i32 == err { Error::CMapTableMissing }
		else if ffi::FT_Err_Hmtx_Table_Missing as i32 == err { Error::HmtxTableMissing }
		else if ffi::FT_Err_Post_Table_Missing as i32 == err { Error::PostTableMissing }
		else if ffi::FT_Err_Invalid_Horiz_Metrics as i32 == err { Error::InvalidHorizMetrics }
		else if ffi::FT_Err_Invalid_CharMap_Format as i32 == err { Error::InvalidCharMapFormat }
		else if ffi::FT_Err_Invalid_PPem as i32 == err { Error::InvalidPPem }
		else if ffi::FT_Err_Invalid_Vert_Metrics as i32 == err { Error::InvalidVertMetrics }
		else if ffi::FT_Err_Could_Not_Find_Context as i32 == err { Error::CouldNotFindContext }
		else if ffi::FT_Err_Invalid_Post_Table_Format as i32 == err { Error::InvalidPostTableFormat }
		else if ffi::FT_Err_Invalid_Post_Table as i32 == err { Error::InvalidPostTable }
		else if ffi::FT_Err_Syntax_Error as i32 == err { Error::Syntax }
		else if ffi::FT_Err_Stack_Underflow as i32 == err { Error::StackUnderflow }
		else if ffi::FT_Err_Ignore as i32 == err { Error::Ignore }
		else if ffi::FT_Err_No_Unicode_Glyph_Name as i32 == err { Error::NoUnicodeGlyphName }
		else if ffi::FT_Err_Missing_Startfont_Field as i32 == err { Error::MissingStartfontField }
		else if ffi::FT_Err_Missing_Font_Field as i32 == err { Error::MissingFontField }
		else if ffi::FT_Err_Missing_Size_Field as i32 == err { Error::MissingSizeField }
		else if ffi::FT_Err_Missing_Fontboundingbox_Field as i32 == err { Error::MissingFontboundingboxField }
		else if ffi::FT_Err_Missing_Chars_Field as i32 == err { Error::MissingCharsField }
		else if ffi::FT_Err_Missing_Startchar_Field as i32 == err { Error::MissingStartcharField }
		else if ffi::FT_Err_Missing_Encoding_Field as i32 == err { Error::MissingEncodingField }
		else if ffi::FT_Err_Missing_Bbx_Field as i32 == err { Error::MissingBbxField }
		else if ffi::FT_Err_Bbx_Too_Big as i32 == err { Error::BbxTooBig }
		else if ffi::FT_Err_Corrupted_Font_Header as i32 == err { Error::CorruptedFontHeader }
		else if ffi::FT_Err_Corrupted_Font_Glyphs as i32 == err { Error::CorruptedFontGlyphs }
		else if ffi::FT_Err_Max as i32 == err { Error::Max }
		else { Error::Unknown }
	}
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(error::Error::description(self))
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        use self::Error::*;
        match *self {
            Ok                           => "Ok",
            CannotOpenResource           => "Cannot open resource",
            UnknownFileFormat            => "Unknown file format",
            InvalidFileFormat            => "Invalid file format",
            InvalidVersion               => "Invalid version",
            LowerModuleVersion           => "Lower module version",
            InvalidArgument              => "Invalid argument",
            UnimplementedFeature         => "Unimplemented feature",
            InvalidTable                 => "Invalid table",
            InvalidOffset                => "Invalid offset",
            ArrayTooLarge                => "Array too large",
            MissingModule                => "Missing module",
            MissingProperty              => "Missing property",
            InvalidGlyphIndex            => "Invalid glyph index",
            InvalidCharacterCode         => "Invalid character code",
            InvalidGlyphFormat           => "Invalid glyph format",
            CannotRenderGlyph            => "Cannot render glyph",
            InvalidOutline               => "Invalid outline",
            InvalidComposite             => "Invalid composite",
            TooManyHints                 => "Too many hints",
            InvalidPixelSize             => "Invalid pixel size",
            InvalidHandle                => "Invalid handle",
            InvalidLibraryHandle         => "Invalid library handle",
            InvalidDriverHandle          => "Invalid driver handle",
            InvalidFaceHandle            => "Invalid face handle",
            InvalidSizeHandle            => "Invalid size handle",
            InvalidSlotHandle            => "Invalid slot handle",
            InvalidCharMapHandle         => "Invalid char map handle",
            InvalidCacheHandle           => "Invalid cache handle",
            InvalidStreamHandle          => "Invalid stream handle",
            TooManyDrivers               => "Too many drivers",
            TooManyExtensions            => "Too many extensions",
            OutOfMemory                  => "Out of memory",
            UnlistedObject               => "Unlisted object",
            CannotOpenStream             => "Cannot open stream",
            InvalidStreamSeek            => "Invalid stream seek",
            InvalidStreamSkip            => "Invalid stream skip",
            InvalidStreamRead            => "Invalid stream read",
            InvalidStreamOperation       => "Invalid stream operation",
            InvalidFrameOperation        => "Invalid frame operation",
            NestedFrameAccess            => "Nested frame access",
            InvalidFrameRead             => "Invalid frame read",
            RasterUninitialized          => "Raster uninitialized",
            RasterCorrupted              => "Raster corrupted",
            RasterOverflow               => "Raster overflow",
            RasterNegativeHeight         => "Raster negative height",
            TooManyCaches                => "Too many caches",
            InvalidOpcode                => "Invalid opcode",
            TooFewArguments              => "Too few arguments",
            StackOverflow                => "Stack overflow",
            CodeOverflow                 => "Code overflow",
            BadArgument                  => "Bad argument",
            DivideByZero                 => "Divide by zero",
            InvalidReference             => "Invalid reference",
            DebugOpCode                  => "Debug op code",
            ENDFInExecStream             => "ENDF in exec stream",
            NestedDEFS                   => "Nested DEFS",
            InvalidCodeRange             => "Invalid code range",
            ExecutionTooLong             => "Execution too long",
            TooManyFunctionDefs          => "Too many function defs",
            TooManyInstructionDefs       => "Too many instruction defs",
            TableMissing                 => "Table missing",
            HorizHeaderMissing           => "Horiz header missing",
            LocationsMissing             => "Locations missing",
            NameTableMissing             => "Name table missing",
            CMapTableMissing             => "C map table missing",
            HmtxTableMissing             => "Hmtx table missing",
            PostTableMissing             => "Post table missing",
            InvalidHorizMetrics          => "Invalid horiz metrics",
            InvalidCharMapFormat         => "Invalid char map format",
            InvalidPPem                  => "Invalid p pem",
            InvalidVertMetrics           => "Invalid vert metrics",
            CouldNotFindContext          => "Could not find context",
            InvalidPostTableFormat       => "Invalid post table format",
            InvalidPostTable             => "Invalid post table",
            Syntax                       => "Syntax",
            StackUnderflow               => "Stack underflow",
            Ignore                       => "Ignore",
            NoUnicodeGlyphName           => "No unicode glyph name",
            MissingStartfontField        => "Missing startfont field",
            MissingFontField             => "Missing font field",
            MissingSizeField             => "Missing size field",
            MissingFontboundingboxField  => "Missing fontboundingbox field",
            MissingCharsField            => "Missing chars field",
            MissingStartcharField        => "Missing startchar field",
            MissingEncodingField         => "Missing encoding field",
            MissingBbxField              => "Missing bbx field",
            BbxTooBig                    => "Bbx too big",
            CorruptedFontHeader          => "Corrupted font header",
            CorruptedFontGlyphs          => "Corrupted font glyphs",
            Max                          => "Max",
            UnexpectedPixelMode          => "Unexpected pixel mode",
            InvalidPath                  => "Invalid path",
            Unknown                      => "Unknown",
        }
    }
}