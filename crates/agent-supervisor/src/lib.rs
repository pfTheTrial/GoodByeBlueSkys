use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentWarmState {
    Cold,
    Warm,
    Hot,
}

#[derive(Debug, Default)]
pub struct AgentSupervisor {
    states_by_agent_id: HashMap<String, AgentWarmState>,
}

impl AgentSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_agent(&mut self, agent_id: String) {
        self.states_by_agent_id
            .entry(agent_id)
            .or_insert(AgentWarmState::Cold);
    }

    pub fn get_state(&self, agent_id: &str) -> Option<AgentWarmState> {
        self.states_by_agent_id.get(agent_id).copied()
    }

    pub fn set_warm(&mut self, agent_id: &str) -> Result<(), SupervisorError> {
        self.set_state(agent_id, AgentWarmState::Warm)
    }

    pub fn set_hot(&mut self, agent_id: &str) -> Result<(), SupervisorError> {
        self.set_state(agent_id, AgentWarmState::Hot)
    }

    pub fn set_cold(&mut self, agent_id: &str) -> Result<(), SupervisorError> {
        self.set_state(agent_id, AgentWarmState::Cold)
    }

    fn set_state(
        &mut self,
        agent_id: &str,
        next_state: AgentWarmState,
    ) -> Result<(), SupervisorError> {
        let Some(current_state) = self.states_by_agent_id.get_mut(agent_id) else {
            return Err(SupervisorError::UnknownAgent(agent_id.to_string()));
        };

        *current_state = next_state;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorError {
    UnknownAgent(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_agent_across_cold_warm_hot() {
        let mut supervisor = AgentSupervisor::new();
        supervisor.register_agent("coding-agent".to_string());

        assert_eq!(
            supervisor.get_state("coding-agent"),
            Some(AgentWarmState::Cold)
        );

        supervisor
            .set_warm("coding-agent")
            .expect("warm transition must work");
        assert_eq!(
            supervisor.get_state("coding-agent"),
            Some(AgentWarmState::Warm)
        );

        supervisor
            .set_hot("coding-agent")
            .expect("hot transition must work");
        assert_eq!(
            supervisor.get_state("coding-agent"),
            Some(AgentWarmState::Hot)
        );

        supervisor
            .set_cold("coding-agent")
            .expect("cold transition must work");
        assert_eq!(
            supervisor.get_state("coding-agent"),
            Some(AgentWarmState::Cold)
        );
    }

    #[test]
    fn returns_error_for_unknown_agent() {
        let mut supervisor = AgentSupervisor::new();
        let result = supervisor.set_hot("missing-agent");

        assert_eq!(
            result,
            Err(SupervisorError::UnknownAgent("missing-agent".to_string()))
        );
    }
}

