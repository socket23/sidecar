//! Mechas here are agents with some task assigned to them
//! They can independently do things with other mechas and work
//! in a very collaborative fashion. The goal would be multi-fold here
//! but to summarize, these meachs can have ownership of resources or leave
//! their marks somewhere, and can take actions and use tools and keep on the task
//! Once they have finished their job (first mechas notify about the job being
//! done and then finish it ALWAYS)
//! Mechas also have their own memory as well

pub mod basic;
pub mod events;
pub mod journal;
pub mod types;
