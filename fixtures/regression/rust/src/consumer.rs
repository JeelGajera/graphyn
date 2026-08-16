use crate::alpha::Alpha;
use crate::beta::Beta;

pub fn use_alpha(whatever: &Alpha) -> usize { whatever.a_field.len() }
pub fn use_beta(anything: &Beta) -> usize { anything.b_field.len() }
