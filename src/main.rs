extern crate mtaillib;
use mtaillib::start_all_tails_internal;

fn main() {
  let _tails = start_all_tails_internal(vec!["/var/log/audit/audit.log".to_string(), "/var/log/secure".to_string()]);
}
