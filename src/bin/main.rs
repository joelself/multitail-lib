extern crate mtaillib;
use mtaillib::mtail::MultiTail;
use std::io::{self, Write};

fn main() {
	let mut m = MultiTail::new(vec!["/Users/joel.self/Projects/joel/test.log".to_string(),
												 "/Users/joel.self/Projects/joel/test2.log".to_string()]);
	loop {
		let msgs = m.wait_for_lines();
		for (thread, s) in msgs {
			print!("File {} => {}", thread, String::from_utf8(s).unwrap());
			io::stdout().flush().unwrap();
		}
	}
}