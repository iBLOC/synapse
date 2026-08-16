
#[derive(Clone, Copy)]
pub enum NodeTriggerMode {
    AllPredecessor,
    AnyPredecessor,
}

impl Default for NodeTriggerMode {
    fn default() -> Self {
        Self::AllPredecessor
    }
}

#[derive(Clone)]
pub struct GraphCompileOptions {
    pub max_steps: usize,
    pub name: String,
    pub interrupt_before_nodes: Vec<String>,
    pub interrupt_after_nodes: Vec<String>,
    pub trigger_mode: NodeTriggerMode,
}

impl Default for GraphCompileOptions {
    fn default() -> Self {
        Self {
            max_steps: 100,
            name: "graph".to_string(),
            interrupt_before_nodes: Vec::new(),
            interrupt_after_nodes: Vec::new(),
            trigger_mode: NodeTriggerMode::default(),
        }
    }
}

pub trait GraphCompileOption: Send + Sync {
    fn apply(&self, opts: &mut GraphCompileOptions);
}

pub struct WithMaxSteps(pub usize);
impl GraphCompileOption for WithMaxSteps {
    fn apply(&self, opts: &mut GraphCompileOptions) {
        opts.max_steps = self.0;
    }
}

pub struct WithGraphName(pub &'static str);
impl GraphCompileOption for WithGraphName {
    fn apply(&self, opts: &mut GraphCompileOptions) {
        opts.name = self.0.to_string();
    }
}

pub struct WithTriggerMode(pub NodeTriggerMode);
impl GraphCompileOption for WithTriggerMode {
    fn apply(&self, opts: &mut GraphCompileOptions) {
        opts.trigger_mode = self.0;
    }
}
