use crate::agentic::action::types::Action;

pub enum MechaJournal {
    MechaCreated,
    AddAction(Action),
    ChangeState,
    MechaEvents,
}
