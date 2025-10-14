/// Ports module - Defines the interfaces (traits) that abstract platform-specific functionality.
///
/// This module contains all the port traits that define contracts between the domain layer
/// and the infrastructure adapters. These traits enable the hexagonal architecture by
/// decoupling the business logic from platform-specific implementations.

pub mod clock;
pub mod logger;

pub use clock::ClockPort;
pub use logger::LoggerPort;
