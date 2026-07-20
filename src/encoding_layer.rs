pub mod anchor;
pub mod indent;
pub mod pipeline;
pub mod traits;

pub use anchor::AnchorEncoder;
pub use indent::IndentEncoder;
pub use pipeline::Pipeline;

use crate::config::GlobalConfig;

pub fn build_pipeline(config: &GlobalConfig) -> Pipeline {
    let mut pipeline = Pipeline::new();
    if config.indent_encoding {
        pipeline.add_stage(IndentEncoder::new(config.tab_width));
    }
    if config.anchor_interval > 0 {
        pipeline.add_stage(AnchorEncoder::new(config.anchor_interval));
    }
    pipeline
}

pub fn build_content_pipeline(config: &GlobalConfig) -> Pipeline {
    let mut pipeline = Pipeline::new();
    if config.indent_encoding {
        pipeline.add_stage(IndentEncoder::new(config.tab_width));
    }
    pipeline
}
