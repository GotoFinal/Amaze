#[derive(Debug, Clone, Copy)]
pub enum Buffering {
    Double,
    Triple,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Multisampling {
    Disable,
    Sample2,
    Sample4,
    Sample8,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicOptions {
    pub multisampling: Multisampling,
    pub buffering: Buffering,
}

impl GraphicOptions {
    pub(crate) fn default() -> Self {
        GraphicOptions {
            multisampling: Multisampling::Sample2,
            buffering: Buffering::Triple,
        }
    }
}