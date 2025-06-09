pub mod post_conditions;
pub mod pre_conditions;

pub use post_conditions::*;
pub use pre_conditions::*;

use super::CommandSpecification;

const PRE_CONDITION: &str = "pre_condition";
const POST_CONDITION: &str = "post_condition";
const BEHAVIOR: &str = "behavior";
const ASSERTION: &str = "assertion";
const RETRIES: &str = "retries";
const BACKOFF: &str = "backoff";
const POST_CONDITION_ATTEMPTS: &str = "post_condition_attempts";
