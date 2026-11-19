//! Native declarative resource management. Runtime/UI integration follows the strict store boundary.
mod appearance_history;
mod author;
mod center;
mod profiles;
pub(crate) mod runtime;
mod storage;
mod widgets;
pub(crate) use center::State;
pub(crate) use profiles::Action as ProfileAction;
