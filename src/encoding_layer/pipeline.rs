use super::traits::TransformStage;

pub struct Pipeline {
    stages: Vec<Box<dyn TransformStage>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: impl TransformStage + 'static) {
        self.stages.push(Box::new(stage));
    }

    pub fn encode_all(&self, input: &str) -> String {
        let mut current = input.to_string();
        for stage in &self.stages {
            current = stage.encode(&current);
        }
        current
    }

    pub fn decode_all(&self, input: &str) -> String {
        let mut current = input.to_string();
        for stage in self.stages.iter().rev() {
            current = stage.decode(&current);
        }
        current
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding_layer::{AnchorEncoder, IndentEncoder};

    #[test]
    fn test_pipeline_round_trip() {
        let mut pipeline = Pipeline::new();
        pipeline.add_stage(IndentEncoder::new(4));
        pipeline.add_stage(AnchorEncoder::new(10));

        let original = "fn main() {\n    let x = 1;\n}\n";
        let encoded = pipeline.encode_all(original);
        let decoded = pipeline.decode_all(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_pipeline_large_file_round_trip() {
        let mut pipeline = Pipeline::new();
        pipeline.add_stage(IndentEncoder::new(4));
        pipeline.add_stage(AnchorEncoder::new(10));

        let lines: Vec<String> =
            (1..=120).map(|i| format!("{}line {}", "    ".repeat(i % 5), i)).collect();
        let original = lines.join("\n") + "\n";
        let encoded = pipeline.encode_all(&original);
        let decoded = pipeline.decode_all(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_empty_pipeline() {
        let pipeline = Pipeline::new();
        let text = "hello world\n";
        assert_eq!(pipeline.encode_all(text), text);
        assert_eq!(pipeline.decode_all(text), text);
    }
}
