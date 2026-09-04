pub mod payload;

// Re-exported one level up, so a consumer naming `models::UserPayload` has to
// be followed through this `pub use` to reach the declaring module.
pub use payload::UserPayload;
