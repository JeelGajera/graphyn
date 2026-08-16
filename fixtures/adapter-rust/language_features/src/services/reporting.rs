use crate::models::order::Order;
use crate::models::user::{Identify, UserPayload as Customer};
use crate::models::*;

pub struct Reporter;

impl Reporter {
    pub fn summarize(&self, customer: &Customer, order: &Order) -> String {
        format!("{} {} {}", customer.email, customer.user_id, order.total)
    }

    pub fn label<T: Identify>(&self, subject: &T) -> String {
        subject.identity()
    }
}
