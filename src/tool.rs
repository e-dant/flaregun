pub trait Tool: futures::Stream {
    type Cfg: Clone + Copy;
    fn try_new(cfg: Option<Self::Cfg>) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}
