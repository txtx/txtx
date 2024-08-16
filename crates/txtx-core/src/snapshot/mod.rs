pub struct Snapshot {
    pub name: String,
    pub state_transitions: Vec<StateTransition>,
}

pub struct StateTransition {
    pub id: u32,
}

// Step 1: Serialize the state of a Runbook Execution
// Step 2: Execute a runbook

// Todo:
// Serialize execution graph
// Load graph
// Introduce snapshot construct in .tx
// Ability to diff 2 graphs, and be smart about it
