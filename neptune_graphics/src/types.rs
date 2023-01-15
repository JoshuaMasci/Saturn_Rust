use crate::{Buffer, Texture};
use bitflags::bitflags;

pub enum Error {}
pub type Result<T> = std::result::Result<T, Error>;

pub type HandleType = u64;

pub type SurfaceHandle = HandleType;
pub type BufferHandle = HandleType;
pub type TextureHandle = HandleType;
pub type SamplerHandle = HandleType;
pub type ComputePipelineHandle = HandleType;
pub type RasterPipelineHandle = HandleType;
pub type SwapchainHandle = HandleType;

bitflags! {
    pub struct BufferUsage: u32 {
        const VERTEX = 1 << 0;
        const INDEX = 1 << 1;
        const UNIFORM = 1 << 2;
        const STORAGE = 1 << 3;
        const INDIRECT  = 1 << 4;
    }
}

#[derive(Debug, Clone)]
pub struct BufferDescription {
    pub size: u64,
    pub usage: BufferUsage,
}

bitflags! {
    pub struct TextureUsage: u32 {
        const ATTACHMENT = 1 << 0;
        const SAMPLED = 1 << 1;
        const STORAGE = 1 << 2;
    }
}

//TODO: Add BC formats + 10 Bit formats + etc (Use WGPU format list as ref?)
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum TextureFormat {
    //Color Formats
    R8Unorm,
    Rg8Unorm,
    Rgb8Unorm,
    Rgba8Unorm,

    R8Snorm,
    Rg8Snorm,
    Rgb8Snorm,
    Rgba8Snorm,

    R8Uint,
    Rg8Uint,
    Rgb8Uint,
    Rgba8Uint,

    R8Sint,
    Rg8Sint,
    Rgb8Sint,
    Rgba8Sint,

    R16Unorm,
    Rg16Unorm,
    Rgb16Unorm,
    Rgba16Unorm,

    R16Snorm,
    Rg16Snorm,
    Rgb16Snorm,
    Rgba16Snorm,

    R16Uint,
    Rg16Uint,
    Rgb16Uint,
    Rgba16Uint,

    R16Sint,
    Rg16Sint,
    Rgb16Sint,
    Rgba16Sint,

    //Depth Stencil Formats
    D16Unorm,
    D24UnormS8Uint,
    D32Float,
    D32FloatS8Uint,
}

impl TextureFormat {
    pub fn is_color(self) -> bool {
        !self.is_depth_stencil()
    }

    pub fn is_depth_stencil(&self) -> bool {
        matches!(
            self,
            TextureFormat::D16Unorm
                | TextureFormat::D24UnormS8Uint
                | TextureFormat::D32Float
                | TextureFormat::D32FloatS8Uint
        )
    }
}

#[derive(Debug, Clone)]
pub struct TextureDescription {
    pub size: [u32; 2],
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub sampler: Option<()>,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum AddressMode {
    #[default]
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum FilterMode {
    #[default]
    Nearest,
    Linear,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum BorderColor {
    #[default]
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}

#[derive(Default, Debug, Clone)]
pub struct SamplerDescription {
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mip_filter: FilterMode,
    pub lod_clamp_range: Option<std::ops::Range<f32>>,
    pub anisotropy_clamp: Option<f32>,
    pub border_color: BorderColor,
    pub unnormalized_coordinates: bool,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ComputePipelineDescription<'a> {
    pub shader: &'a [u32],
}

//TODO: Add complete list from WGPU?
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum VertexFormat {
    Byte,
    Byte2,
    Byte3,
    Byte4,
    Float,
    Float2,
    Float3,
    Float4,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum IndexFormat {
    U16,
    U32,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum VertexStepMode {
    Vertex,
    Instance,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexAttribute {
    pub format: VertexFormat,
    pub offset: u32,
    pub shader_location: u32,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexBufferLayout<'a> {
    pub stride: u32,
    pub step: VertexStepMode,
    pub attributes: &'a [VertexAttribute],
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct VertexState<'a> {
    pub shader: &'a [u32],
    pub layouts: &'a [VertexBufferLayout<'a>],
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum BlendFactor {
    Zero,
    One,
    ColorSrc,
    OneMinusColorSrc,
    ColorDst,
    OneMinusColorDst,
    AlphaSrc,
    OneMinusAlphaSrc,
    AlphaDst,
    OneMinusAlphaDst,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum BlendOperation {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct BlendComponent {
    src_factor: BlendFactor,
    dst_factor: BlendFactor,
    blend_op: BlendOperation,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct BlendState {
    color: BlendComponent,
    alpha: BlendComponent,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ColorTargetState {
    pub format: TextureFormat,
    pub blend: Option<BlendState>,
    pub write_mask: (), //TODO: color writes
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FragmentState<'a> {
    pub shader: &'a [u32],
    pub targets: &'a [ColorTargetState],
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum CompareOperation {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct DepthStencilState {
    pub format: TextureFormat,
    pub write_depth: bool,
    pub depth_op: CompareOperation,
    //TODO: Stencil State and Depth Bias
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum CullMode {
    Front,
    Back,
    All,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct PrimitiveState {
    front_face: FrontFace,
    cull_mode: Option<CullMode>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct RasterPipelineDescription<'a> {
    pub vertex: VertexState<'a>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub fragment: Option<FragmentState<'a>>,
}

#[derive(Default, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum PresentMode {
    #[default]
    Fifo,
    Immediate,
    Mailbox,
}

#[derive(Default, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum CompositeAlphaMode {
    #[default]
    Auto,
    Opaque,
    PreMultiplied,
    PostMultiplied,
    Inherit,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct SwapchainDescription {
    pub format: TextureFormat,
    pub present_mode: PresentMode,
    pub usage: TextureUsage,
    pub composite_alpha: CompositeAlphaMode,
}

#[derive(Debug, Copy, Clone, Hash)]
pub enum Queue {
    Primary,
    PreferAsyncCompute,
    PreferAsyncTransfer,
}

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct BufferGraphResource(usize);

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct TextureGraphResource(usize);

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum ShaderResourceAccess {
    BufferUniformRead(BufferGraphResource),
    BufferStorageRead(BufferGraphResource),
    BufferStorageWrite(BufferGraphResource),
    TextureSampleRead(TextureGraphResource),
    TextureStorageRead(TextureGraphResource),
    TextureStorageWrite(TextureGraphResource),
}

#[derive(Debug, Clone)]
pub struct TextureCopyBuffer {
    buffer: BufferGraphResource,
    offset: u64,
    row_length: Option<u32>,
    row_height: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TextureCopyTexture {
    texture: TextureGraphResource,
    offset: [u32; 2],
}

pub enum Transfer<'a> {
    CopyCpuToBuffer {
        src: &'a [u8],
        dst: BufferGraphResource,
        dst_offset: u64,
        copy_size: u64,
    },
    CopyCpuToTexture {
        src: &'a [u8],
        row_length: Option<u32>,
        row_height: Option<u32>,
        dst: TextureCopyTexture,
        copy_size: [u32; 2],
    },
    CopyBufferToBuffer {
        src: BufferGraphResource,
        src_offset: u64,
        dst: BufferGraphResource,
        dst_offset: u64,
        copy_size: u64,
    },
    CopyBufferToTexture {
        src: TextureCopyBuffer,
        dst: TextureCopyTexture,
        copy_size: [u32; 2],
    },
    CopyTextureToBuffer {
        src: TextureCopyTexture,
        dst: TextureCopyBuffer,
        copy_size: [u32; 2],
    },
    CopyTextureToTexture {
        src: TextureCopyTexture,
        dst: TextureCopyTexture,
        copy_size: [u32; 2],
    },
}

#[derive(Debug, Clone)]
pub enum ComputeDispatch {
    Size([u32; 3]),
    Indirect {
        buffer: BufferGraphResource,
        offset: u64,
    },
}

#[derive(Debug, Clone)]
pub struct ColorAttachment {
    texture: TextureGraphResource,
    clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(texture: TextureGraphResource) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: TextureGraphResource, clear: [f32; 4]) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepthStencilAttachment {
    texture: TextureGraphResource,
    clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(texture: TextureGraphResource) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: TextureGraphResource, clear: (f32, u32)) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct RasterPassDescription<'a> {
    color_attachments: &'a [ColorAttachment],
    depth_stencil_attachment: Option<DepthStencilAttachment>,
    input_attachments: &'a [TextureGraphResource],
}
