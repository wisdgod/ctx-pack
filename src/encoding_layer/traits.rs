pub trait TransformStage {
    fn encode(&self, input: &str) -> String;
    fn decode(&self, input: &str) -> String;
}
