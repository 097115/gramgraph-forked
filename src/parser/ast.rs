// Abstract Syntax Tree definitions for PlotPipe DSL

#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Chart {
        x: String,
        y: String,
        title: Option<String>,
    },
    LayerLine {
        color: Option<String>,
        stroke: Option<u32>,
    },
    LayerPoint {
        shape: Option<String>,
        size: Option<u32>,
        color: Option<String>,
    },
}
