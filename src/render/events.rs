
/// Events emitted during WFC generation that renderers can handle
#[derive(Debug, Clone)]
pub enum RenderEvent {
    /// Generation started
    Started,
    
    /// Progress update with current state
    Progress,
    
    /// Generation completed successfully  
    Completed,
}

