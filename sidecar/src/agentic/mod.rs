//! The agents and humans can take various actions and interact with each other
//! We need to model this on some level
//! Some ideas:
//! - The agent can decide what tool to use
//! - Each new agent task is well scoped and defined and only certain agents can
//! do heavy work
//! - The human can interact with the agent(s) in any way as required
//! - There is some quantum of work which can be done by each agent, we define
//! this as action
//! - Each action will also have a memory state which will be used through-out the
//! execution of the action
//! - Each action can spawn other actions which are inter-connected together, allowing bigger changes
//! to happen
//! - Each action can either be stopped because of user-input somehow or be complete or failed or in-process or human has taken over
//! - Each action has a dependency on other actions, awaiting for their finish state to be reached
//! - There is an environment where all of this happens, we need to model this somehow
//! - The human can spawn off other agents or the agent (the big one can also spawn other agents as and when required)
//!
//!
//! Nomenclature (cause we keep things professional here, but everyone loves anime and I hate paying tech-debt)
//! agent == mecha
mod action;
mod environment;
mod mecha;
mod memory;
mod tool;
